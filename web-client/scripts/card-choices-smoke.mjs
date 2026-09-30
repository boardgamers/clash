import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'choice-smoke', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return JSON.stringify(game);
}
const move = (state, action) =>
  engine.tryMove(
    state,
    typeof action === 'string' ? action : JSON.stringify(action),
    engine.currentPlayer(state),
  );
let seer = await fixture('incidents/great_persons/great_seer');
for (const action of [
  { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } },
  { Response: { Payment: [{ culture_tokens: 1 }] } },
  { Playing: { ActionCard: 158 } },
])
  seer = move(seer, action);
let spy = await fixture('action_cards/spy');
spy = move(move(spy, { Playing: { ActionCard: 7 } }), { Response: { Payment: [{ culture_tokens: 1 }] } });
const economy = await fixture('advances/collect_free_economy');
const server = createServer(async (req, res) => {
  if (req.url === '/')
    return res.end(
      '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
    );
  if (!['/viewer.js', '/server_bg.wasm'].includes(req.url)) return res.writeHead(404).end();
  res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
  res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height, dark] of [
    [1400, 900, false],
    [390, 844, true],
    [320, 640, false],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    let state = seer,
      sent = 0;
    page.on('pageerror', (error) => errors.push(error.message));
    await page.exposeFunction('applyMove', async (action) => {
      state = move(state, action);
      sent++;
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state, 0));
    });
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      ({ state, dark }) => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: 0 });
        host.emit('theme', { dark });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: engine.stripSecret(state, 0), dark },
    );
    const panel = page.getByRole('region', { name: 'Great Seer', exact: true });
    await panel.waitFor();
    assert.match(await panel.innerText(), /The other card goes to Greece on their next objective draw/);
    assert.equal(await panel.locator('.decision-card-face').count(), 4);
    assert.equal(await panel.locator('.decision-alternative').count(), 2);
    assert.equal(await panel.getByRole('button', { name: /About / }).count(), 0);
    assert.equal(await panel.locator('details').count(), 0);
    assert.ok(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(
      await panel.locator('.decision-options').evaluate((el) => el.scrollHeight <= el.clientHeight + 1),
    );
    const first = panel.locator('.decision-option').first();
    await first.click();
    assert.equal(await first.getAttribute('aria-pressed'), 'true');
    assert.equal(await panel.getByRole('button', { name: 'Confirm', exact: true }).isEnabled(), true);
    await panel.evaluate((el) => {
      el.scrollTop = 0;
    });
    await page.screenshot({ path: `/tmp/clash-card-choices-${width}.png` });
    await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.card-decision'));
    assert.equal(sent, 1);
    assert.equal(
      JSON.parse(state).permanent_effects.find((e) => e.GreatSeer).GreatSeer.assigned_objectives.length,
      2,
    );
    state = spy;
    await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state, 0));
    const spyPanel = page.getByRole('region', { name: 'Spy', exact: true });
    await spyPanel.waitFor();
    assert.ok(await spyPanel.getByText('Battle · Requires Tactics', { exact: true }).count());
    assert.ok(await spyPanel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await spyPanel.screenshot({ path: `/tmp/clash-spy-choices-${width}.png` });
    state = economy;
    sent = 0;
    await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state, 0));
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    await page.locator('.variant-picker button').filter({ hasText: 'Free Economy' }).click();
    const tiles = page.locator('.collection-tile-list');
    await tiles.locator('summary').click();
    await tiles.locator('button').first().click();
    await page.locator('.collect-submit').click();
    await page.waitForFunction(
      () => !document.querySelector('.board-collection') && !document.querySelector('.decision-panel'),
    );
    // Both engine messages result from the single Collect click.
    for (let tries = 0; sent < 2 && tries < 50; tries++) await page.waitForTimeout(100);
    assert.equal(sent, 2);
    assert.equal(
      JSON.parse(state).players[0].resources.mood_tokens,
      JSON.parse(economy).players[0].resources.mood_tokens - 1,
    );
    assert.deepEqual(errors, []);
    console.log(`${width}px: full card choices, recipient, selection, and one-click Free Economy verified.`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
