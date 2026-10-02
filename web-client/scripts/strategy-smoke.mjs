import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { militarySummary, strategyTiles, strategyDescription } from '../src/strategy.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'strategy-smoke', {});
for (const civ of ['Maya', 'Carthage'])
  raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(raw));
const game = JSON.parse(raw),
  seat = game.current_player_index;
game.map.unexplored_blocks = [];
game.map.tiles = game.map.tiles.map(([p, t], i) => [
  p,
  t === 'Unexplored' ? ['Fertile', 'Mountain', 'Forest', 'Barren'][i % 4] : t,
]);
const water = ['E4', 'F4', 'F6'];
for (const tile of game.map.tiles) if (water.includes(tile[0])) tile[1] = 'Water';
let id = 0;
const units = (position, types) => types.map((unit_type) => ({ id: id++, position, unit_type }));
game.players[0].units = [
  ...units('D2', ['Infantry', 'Infantry', 'Cavalry', 'Elephant']),
  ...units('C2', ['Infantry', 'Settler']),
  ...units('E4', ['Ship']),
];
game.players[0].units.at(-1).carried_units = units('E4', ['Infantry', 'Cavalry']).map(
  ({ id, unit_type }) => ({ id, unit_type }),
);
game.players[1].units = [
  ...units('D7', ['Infantry', 'Infantry', 'Cavalry']),
  ...units('E7', ['Elephant', 'Settler']),
  ...units('F4', ['Ship']),
];
for (const p of game.players.slice(0, 2)) {
  p.next_unit_id = id + 10;
  p.advances = ['Farming', 'Mining', 'Storage', 'Tactics', 'Fishing', 'Navigation'];
  p.resources = { food: 4, ore: 3, wood: 3, mood_tokens: 3, culture_tokens: 3 };
  p.cities[0].city_pieces = { fortress: p.id };
}
game.players[2].units = units('B4', ['Infantry', 'Infantry', 'Infantry']);
game.players[2].cities = [{ position: 'B4', mood_state: 'Angry', city_pieces: {} }];
game.players[3].units = units('F6', ['Ship', 'Ship']);
delete game.board_history;
raw = engine.stripSecret(JSON.stringify(game), seat);
const expected = strategyTiles(JSON.parse(raw));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 780],
    [320, 640],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [],
      sent = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('moveSent', (a) => sent.push(a));
    await page.route('**/*', async (route) => {
      const p = new URL(route.request().url()).pathname;
      if (p === '/')
        return route.fulfill({
          contentType: 'text/html',
          body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
        });
      if (!['/viewer.js', '/server_bg.wasm'].includes(p)) return route.abort();
      return route.fulfill({
        contentType: p.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
        body: await readFile(new URL('../dist' + p, import.meta.url)),
      });
    });
    await page.goto('http://clash.test/');
    await page.evaluate(
      ({ raw, seat }) => {
        window.host = clash3d.launch('#app');
        window.prefs = { sound: false };
        host.on('move', (a) => window.moveSent(a));
        host.on('update:preference', ({ name, value }) => {
          prefs[name] = value;
          host.emit('preferences', prefs);
        });
        host.emit('player', { index: seat });
        host.emit('preferences', prefs);
        host.emit('theme', { dark: true });
        host.emit('state', raw);
      },
      { raw, seat },
    );
    const mode = page.getByRole('button', { name: 'Strategy map', exact: true });
    assert.equal(await mode.getAttribute('aria-pressed'), 'false');
    assert.equal(await page.getByRole('combobox', { name: 'Map view', exact: true }).count(), 0);
    assert.equal(await page.getByRole('button', { name: 'Unit badges', exact: true }).count(), 0);
    assert.equal(await page.getByRole('button', { name: 'Reset camera', exact: true }).count(), 0);
    await mode.click();
    assert.equal(await mode.getAttribute('aria-pressed'), 'true');
    await page.locator('.strategy-map-tile').first().waitFor();
    assert.equal(await page.locator('.strategy-map-tile').count(), expected.length);
    for (const tile of expected.filter((t) => t.occupants.length))
      assert.equal(
        await page.locator(`.strategy-map-tile[data-position="${tile.position}"]`).getAttribute('aria-label'),
        strategyDescription(tile),
      );
    const army = militarySummary(game.players[0]);
    assert.equal(army.army, 7);
    assert.equal(army.aboard, 2);
    assert(await page.getByLabel('7 army units, 1 ships, 2 army aboard', { exact: true }).isVisible());
    assert.equal(await page.locator('.strategy-map-tile[data-position="F6"] svg.lucide-skull').count(), 1);
    assert.doesNotMatch(
      (await page.locator('.strategy-map-tile').allTextContents()).join(' '),
      /#[0-9]|\b[A-Z][0-9]/,
    );
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    await page.screenshot({ path: `/tmp/clash-strategy-${width}.png` });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.getByLabel('Strategy map key', { exact: true }).click();
    const key = page.locator('.strategy-key-content');
    assert(await key.isVisible());
    const keyBounds = await key.boundingBox();
    assert(keyBounds.x >= 0 && keyBounds.x + keyBounds.width <= width);
    assert.match(await key.innerText(), /Terrain adds no combat value/);
    await page.getByLabel('Strategy map key', { exact: true }).click();

    await page.locator('.strategy-map-tile[data-position="B4"]').click();
    await page.getByRole('region', { name: 'Tile B4 actions' }).waitFor();
    await page.getByRole('button', { name: 'Close tile actions', exact: true }).click();
    await page.getByRole('button', { name: 'Show all sea routes', exact: true }).click();
    assert.equal(await mode.getAttribute('aria-pressed'), 'true');
    await page.getByRole('button', { name: 'Show all sea routes', exact: true }).click();
    await mode.click();
    assert.equal(await mode.getAttribute('aria-pressed'), 'false');
    assert.equal(await page.locator('.strategy-map-tile').count(), 0);
    await page.evaluate(() => {
      prefs.mapView = '2d';
      host.emit('preferences', prefs);
    });
    await page.locator('.strategy-map-tile').first().waitFor();
    assert.equal(await mode.getAttribute('aria-pressed'), 'true', 'legacy 2D opens Strategy');
    await mode.click();
    assert.equal(await page.evaluate(() => prefs.mapView), '3d');
    await mode.click();
    assert.equal(await page.evaluate(() => prefs.mapView), 'strategy');

    await page.getByRole('button', { name: 'Enter fullscreen', exact: true }).click();
    await page.waitForFunction(() => document.fullscreenElement === document.documentElement);
    await page.getByRole('button', { name: 'Exit fullscreen', exact: true }).click();
    await page.waitForFunction(() => !document.fullscreenElement);
    await page.getByRole('button', { name: 'Enter fullscreen', exact: true }).click();
    await page.waitForFunction(() => document.fullscreenElement === document.documentElement);
    // Browser controls / Escape can exit without using our button.
    await page.evaluate(() => document.exitFullscreen());
    await page.getByRole('button', { name: 'Enter fullscreen', exact: true }).waitFor();
    await page.evaluate(() => {
      prefs.colorBlind = true;
      host.emit('preferences', prefs);
    });
    assert((await page.locator('.strategy-symbol').count()) > 0);
    const own = game.players[seat].cities[0].position;
    await page.locator(`.strategy-map-tile[data-position="${own}"]`).click();
    assert.deepEqual(sent, [], 'inspection and map views never submit moves');
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: strategy ownership, army/ship/passenger counts, pirates, color symbols, sea helper, legacy preference, view toggle and fullscreen verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
