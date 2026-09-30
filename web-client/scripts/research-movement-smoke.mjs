import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = async (name) =>
  JSON.parse(await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
const npcs = JSON.parse(await engine.init(2, [], {}, 'research-smoke', {})).players.slice(2);
const withNpcs = (g) => {
  for (const npc of npcs)
    if (!g.players.some((p) => p.civilization === npc.civilization))
      g.players.push({ ...npc, id: g.players.length });
  return g;
};
const ser = (g) => (typeof g === 'string' ? g : JSON.stringify(g));
const move = (g, action) =>
  engine.tryMove(
    ser(g),
    typeof action === 'string' ? action : JSON.stringify(action),
    engine.currentPlayer(ser(g)),
  );
const server = createServer(async (req, res) => {
  if (req.url === '/')
    return res.end(
      '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
    );
  if (!['/viewer.js', '/server_bg.wasm'].includes(req.url)) return res.writeHead(404).end();
  res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
  res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of process.argv.includes('--movement-only') ? [] : [1400, 390]) {
    for (const dark of [false, true]) {
      let state = move(withNpcs(await fixture('action_cards/synergies')), { Playing: { ActionCard: 34 } });
      const page = await browser.newPage({ viewport: { width, height: 900 } }),
        errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.exposeFunction('applyMove', async (action) => {
        state = move(state, action);
        await page.evaluate(
          (state) => host.emit('state', state),
          engine.stripSecret(state, engine.currentPlayer(state)),
        );
      });
      await page.goto('http://127.0.0.1:' + server.address().port);
      await page.evaluate(
        ({ state, dark }) => {
          window.host = clash3d.launch('#app');
          window.ready = false;
          host.on('ready', () => (window.ready = true));
          host.on('move', (move) => window.applyMove(move));
          host.emit('player', { index: 0 });
          host.emit('preferences', { sound: false });
          host.emit('theme', { dark });
          host.emit('state', state);
        },
        { state: engine.stripSecret(state, 0), dark },
      );
      await page.waitForFunction(() => window.ready);
      for (const [id, label] of [
        ['Fishing', 'Fishing'],
        ['WarShips', 'War Ships'],
      ]) {
        const dialog = page.getByRole('dialog', { name: 'Synergies' });
        await dialog.waitFor();
        await dialog.locator(`#research-${id} .research-pick`).click();
        assert.match(await dialog.locator(`#research-${id} .research-node-cost`).innerText(), /2/);
        assert.match(
          await dialog.locator('.research-payment .primary').innerText(),
          /Pay research cost next/,
        );
        await dialog.locator('.research-payment .primary').click();
        await page.waitForFunction(() => !document.querySelector('.research-dialog'));
        // Payment is still an engine-validated follow-up; advance through the same host channel.
        await page.evaluate(() => window.applyMove({ Response: { Payment: [{ ideas: 2 }] } }));
      }
      await page.waitForFunction(() => !document.querySelector('.research-dialog'));
      let philosophy = withNpcs(await fixture('advances/writing'));
      philosophy.players[0].advances = ['Writing', 'Philosophy'];
      state = ser(philosophy);
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state, 0));
      await page.getByRole('button', { name: 'Research tree', exact: true }).click();
      const science = page.locator('#research-Math');
      await science.scrollIntoViewIfNeeded();
      assert.match(await science.locator('.research-effects').innerText(), /Philosophy/);
      await page.screenshot({ path: `/tmp/clash-research-${width}-${dark ? 'dark' : 'light'}.png` });
      await page.getByRole('button', { name: 'Close research', exact: true }).click();
      state = ser(withNpcs(await fixture('advances/collect_free_economy')));
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state, 0));
      await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
      const economy = page.locator('.variant-picker button').filter({ hasText: 'Free Economy' });
      await economy.click();
      assert.equal(await economy.locator('[aria-label="1 Mood"]').count(), 1);
      const submit = page.locator('.collect-submit');
      assert.match(await submit.getAttribute('aria-label'), /Pay 1 mood/i);
      await page.screenshot({ path: `/tmp/clash-economy-${width}-${dark ? 'dark' : 'light'}.png` });
      assert.deepEqual(errors, []);
      console.log(
        `${width}px ${dark ? 'dark' : 'light'}: Synergies both choices, Philosophy bonus, upfront Free Economy payment.`,
      );
      await page.close();
    }
  }
  if (process.argv[2]) {
    // Optional exported game: inspect controls and move only a local copy.
    let data = JSON.parse(await readFile(process.argv[2], 'utf8')),
      state = data.data ?? ser(data);
    const page = await browser.newPage({ viewport: { width: 390, height: 900 } });
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      (state) => {
        window.host = clash3d.launch('#app');
        host.emit('player', { index: 0 });
        host.emit('preferences', { sound: false });
        host.emit('theme', { dark: true });
        host.emit('state', state);
      },
      engine.stripSecret(state, 0),
    );
    const locations = page.locator('.movement-locations');
    await locations.locator('summary').click();
    await locations.getByRole('button', { name: /C4/ }).click();
    const panel = page.getByRole('region', { name: 'Unit movement' });
    const passenger = panel.getByRole('button', { name: 'Disembark Sun Tzu', exact: true });
    await passenger.click();
    assert.equal(await passenger.getAttribute('aria-pressed'), 'true');
    assert.equal(await panel.locator('.unit-picker button[aria-pressed="true"]').count(), 0);
    assert.match(await panel.locator('h2').innerText(), /Disembark/);
    await panel.locator('.movement-destination-list summary').click();
    await panel.getByRole('button', { name: /C5$/ }).click();
    assert.equal(await panel.getByRole('button', { name: /Disembark at C5/ }).isEnabled(), true);
    await page.screenshot({ path: '/tmp/clash-disembark-390.png' });
    await locations.getByRole('button', { name: /E5/ }).click();
    assert.match(await panel.locator('.movement-notes').innerText(), /Forest · Cannot attack this turn/);
    await page.screenshot({ path: '/tmp/clash-forest-390.png' });
    await page.getByRole('button', { name: 'Close movement controls', exact: true }).click();
    await page.getByRole('button', { name: 'Unit badges', exact: true }).click();
    await page
      .getByRole('button', { name: /Inspect Pirates/ })
      .first()
      .click();
    const pirates = page.getByRole('region', { name: 'Pirate effects', exact: true });
    await pirates.waitFor();
    assert.match(await pirates.innerText(), /1 resource, mood token or culture token in total/);
    assert.ok(await pirates.locator('.pirate-coordinates button').count());
    await page.screenshot({ path: '/tmp/clash-pirates-390.png' });
    await page.getByRole('button', { name: 'Close tile actions', exact: true }).click();
    await page.getByRole('button', { name: /Greece:.*View resources/ }).click();
    await page.getByRole('button', { name: /Cities/ }).click();
    const city = page.getByRole('region', { name: 'City D7', exact: true });
    await city.getByRole('button', { name: 'Academy', exact: true }).click();
    await page.getByRole('region', { name: 'Academy effect', exact: true }).waitFor();
    assert.match(
      await page.getByRole('region', { name: 'Academy effect', exact: true }).innerText(),
      /Gain.*2.*ideas/s,
    );
    await page.screenshot({ path: '/tmp/clash-building-effect-390.png' });
    const q = JSON.parse(
      engine.webQuery(engine.stripSecret(state, 0), 0, JSON.stringify({ kind: 'movement', units: [2] })),
    );
    const landed = JSON.parse(move(state, q.destinations.find((d) => d.position === 'C5').action));
    assert.ok(landed.players[0].units.some((u) => u.id === 2 && u.position === 'C5'));
    console.log(
      'Live export copy: Sun Tzu disembarks successfully; forest attack restriction explained. No live moves submitted.',
    );
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
