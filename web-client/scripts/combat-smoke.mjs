import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'combat-smoke', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return engine.stripSecret(JSON.stringify(game), 0);
}
const pending = await fixture('remove_casualties_attacker.outcome');
const completed = await fixture('combat_all_modifiers.outcome5');
const elephant = await fixture('direct_capture_city_metallurgy.outcome');
const naval = await fixture('ship_combat.outcome');
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
    if (process.env.SMOKE_WIDTH && width !== Number(process.env.SMOKE_WIDTH)) continue;
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      ({ state, dark }) => {
        window.host = clash3d.launch('#app');
        host.emit('player', { index: 0 });
        host.emit('theme', { dark });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: pending, dark },
    );
    const marker = page.locator('.map-combat-label');
    await marker.waitFor();
    assert.equal(await marker.innerText(), 'Battle · C1');
    assert.match(await marker.getAttribute('aria-label'), /Rome attacks Greece · C2 to C1/);
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    const table = page.getByRole('table', { name: 'Combat round 1', exact: true });
    await table.waitFor();
    assert.equal(
      await table
        .locator('tbody tr')
        .filter({ has: page.getByRole('rowheader', { name: 'Value', exact: true }) })
        .innerText(),
      'Value\t26\t14',
    );
    assert.equal(
      await table
        .locator('tbody tr')
        .filter({ has: page.getByRole('rowheader', { name: 'Hits dealt', exact: true }) })
        .innerText(),
      'Hits dealt\t2\t2',
    );
    assert.ok(await table.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.equal(await table.locator('.combat-die').count(), 6);
    assert.equal(await table.locator('.combat-die > svg').count(), 6);
    assert.equal(await table.locator('.combat-die.activated').count(), 4);
    assert.equal(
      await table.getByRole('img', { name: '6 · Infantry face · No ability activated', exact: true }).count(),
      2,
    );
    await page.screenshot({ path: `/tmp/clash-combat-active-${width}.png` });
    await page.evaluate((state) => host.emit('state', state), completed);
    await marker.waitFor({ state: 'hidden' });
    await page.getByText('Rome wins', { exact: true }).waitFor();
    assert.match(await table.innerText(), /Peltasts/);
    assert.match(await table.innerText(), /Encircled/);
    assert.doesNotMatch(await table.innerText(), /Gain Fortress|Lose Fortress/);
    assert.ok(await page.locator('.journal').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-combat-result-${width}.png` });
    await page.evaluate((state) => host.emit('state', state), elephant);
    await table.getByText('Blocks 1 hit').waitFor();
    assert.equal(await table.locator('.rerolled').count(), 1);
    assert.equal(
      await table.getByRole('img', { name: '1 · Leader face · Rerolled', exact: true }).count(),
      1,
    );
    assert.equal(
      await table
        .getByRole('img', { name: '2 · Elephant face · -1 hits, no combat value', exact: true })
        .count(),
      1,
    );
    assert.ok(await table.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: `/tmp/clash-combat-elephant-${width}.png` });
    await page.evaluate((state) => host.emit('state', state), naval);
    await page.waitForFunction(() => document.querySelectorAll('.combat-die').length === 3);
    const journalButton = page.getByRole('button', { name: 'Open journal', exact: true });
    if ((await journalButton.getAttribute('aria-expanded')) === 'false') await journalButton.click();
    await table.waitFor();
    await page.screenshot({ path: `/tmp/clash-combat-naval-${width}.png` });
    assert.equal(
      await table.getByRole('img', { name: '6 · Infantry face · No ability activated', exact: true }).count(),
      3,
    );
    assert.equal(await table.locator('.combat-die.activated').count(), 0);
    assert.equal(await table.locator('.combat-die-effect').count(), 0);
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: combat marker, dice numbers and faces, activated abilities, naval rolls and completed battle verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
