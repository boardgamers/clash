import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'estimate-ui', {})).players.slice(2);
function withNpcs(game) {
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const game = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/combat/direct_capture_city_metallurgy.json', import.meta.url),
    'utf8',
  ),
);
withNpcs(game);
game.players[0].advances = game.players[0].advances.filter((a) => a !== 'Metallurgy');
const pending = engine.tryMove(
  JSON.stringify(game),
  JSON.stringify({ Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1', payment: {} } } }),
  0,
);
const state = engine.stripSecret(pending, 0);
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
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate((state) => {
      window.sent = [];
      window.host = clash3d.launch('#app');
      host.on('move', (a) => window.sent.push(a));
      host.emit('player', { index: 0 });
      host.emit('theme', { dark: true });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    }, state);
    const panel = page.getByRole('region', { name: 'Steel Weapons', exact: true });
    await panel.waitFor();
    await panel.locator('.estimate-summary').waitFor({ timeout: 60000 });
    await panel.getByRole('button', { name: 'Assumptions & options', exact: false }).click();
    await panel.getByText('2048 simulations', { exact: false }).waitFor({ timeout: 60000 });
    assert.match(await panel.innerText(), /Win ≈\d+%/);
    assert.equal(await page.evaluate(() => window.sent.length), 0, 'estimating must never submit a move');
    assert.ok(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/combat-estimate-${width}.png` });
    const attackState = engine.stripSecret(
      JSON.stringify(
        withNpcs(
          JSON.parse(
            await readFile(
              new URL('../../server/tests/test_games/combat/combat_all_modifiers.json', import.meta.url),
              'utf8',
            ),
          ),
        ),
      ),
      0,
    );
    await page.reload();
    await page.evaluate((state) => {
      window.sent = [];
      window.host = clash3d.launch('#app');
      host.on('move', (a) => window.sent.push(a));
      host.emit('player', { index: 0 });
      host.emit('theme', { dark: true });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    }, attackState);
    await page.getByRole('button', { name: 'Move units and found cities' }).click();
    const target = page.locator('.map-hit-target[data-position="C1"]');
    await target.focus();
    await target.press('Enter');
    const attackPanel = page.getByRole('region', { name: 'Unit movement' });
    await attackPanel.locator('.estimate-summary').waitFor({ timeout: 60000 });
    await attackPanel.getByRole('button', { name: 'Assumptions & options', exact: false }).click();
    await attackPanel.getByText('2048 simulations', { exact: false }).waitFor({ timeout: 60000 });
    assert.match(await attackPanel.innerText(), /Cancel both/);
    assert.equal(await page.evaluate(() => window.sent.length), 0);
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/combat-attack-estimate-${width}.png` });
    console.log('Combat estimate UI passed:', width);
    await page.close();
  }
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
