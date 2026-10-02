import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const g = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/movement/ship_navigation_unit_test.json', import.meta.url),
    'utf8',
  ),
);
const npcs = JSON.parse(await engine.init(2, [], {}, 'ship-ui', {})).players.slice(2);
for (const p of npcs)
  if (!g.players.some((q) => q.civilization === p.civilization))
    g.players.push({ ...p, id: g.players.length });
g.state = 'Playing';
g.actions_left = 3;
g.current_player_index = 1;
for (const p of g.players) p.units = [];
g.players[1].units = [
  {
    id: 1,
    unit_type: 'Ship',
    position: 'F6',
    carried_units: [
      { id: 2, unit_type: 'Infantry' },
      { id: 3, unit_type: 'Infantry' },
    ],
  },
];
g.players[1].advances = [...new Set([...g.players[1].advances, 'Tactics', 'Navigation'])];
g.players[0].cities = [{ position: 'F7', mood_state: 'Neutral', city_pieces: {} }];
g.map.tiles = g.map.tiles.map(([p]) => [p, ['F5', 'F6', 'G7', 'A7', 'A3'].includes(p) ? 'Water' : 'Fertile']);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 850 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/*', async (route) => {
      const path = new URL(route.request().url()).pathname;
      if (path === '/')
        return route.fulfill({
          contentType: 'text/html',
          body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
        });
      if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
      return route.fulfill({
        contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
        body: await readFile(new URL('../dist' + path, import.meta.url)),
      });
    });
    await page.goto('http://clash.test/');
    await page.evaluate(
      (raw) => {
        window.sent = [];
        window.host = clash3d.launch('#app');
        host.on('move', (m) => sent.push(m));
        host.emit('player', { index: 1 });
        host.emit('preferences', { sound: false });
        host.emit('theme', { dark: true });
        host.emit('state', raw);
      },
      engine.stripSecret(JSON.stringify(g), 1),
    );
    await page.getByRole('button', { name: 'Move units and found cities' }).click();
    const ship = page.getByRole('button', { name: 'Ship', exact: true });
    if ((await ship.getAttribute('aria-pressed')) !== 'true') await ship.click();
    const far = page.locator('.map-hit-target[data-position="A7"]');
    await far.waitFor({ state: 'attached' });
    const shore = page.locator('.map-hit-target[data-position="F7"]');
    await shore.focus();
    await shore.press('Enter');
    const picker = page.getByRole('group', { name: 'Passengers to disembark' });
    await picker.waitFor();
    assert.equal(await picker.getByRole('button', { pressed: true }).count(), 2);
    assert.equal(await picker.getByRole('button', { name: 'Ship', exact: true }).count(), 0);
    await page.screenshot({ path: `/tmp/clash-landing-${width}.png` });
    await picker.getByRole('button').last().click();
    await page.getByRole('button', { name: /^Attack/ }).click();
    const sent = await page.evaluate(() => sent.map((s) => JSON.parse(s)));
    assert.equal(sent.length, 1);
    const body = sent[0].Playing?.MoveUnits ?? sent[0].Movement?.Move ?? sent[0].Playing?.Move;
    assert.ok(body?.units);
    assert.deepEqual(body.units, [2]);
    assert.equal(body.destination, 'F7');
    assert.doesNotThrow(() => engine.tryMove(JSON.stringify(g), JSON.stringify(sent[0]), 1));
    assert.deepEqual(errors, []);
    console.log(
      `${width}: ship to coastal city opens passenger picker and sends only selected passenger attack.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
