import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
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
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } }),
      errors = [],
      sent = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('moveSent', (a) => sent.push(a));
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
      ({ raw, seat }) => {
        window.host = clash3d.launch('#app');
        window.prefs = { sound: false, unitBadges: true };
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
    const tooltip = page.locator('.tile-hover');
    const ships = page.getByRole('button', { name: /^Inspect .* · E4:/ });
    await ships.hover();
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /Ship/);
    assert.match(await tooltip.innerText(), /Aboard/);
    assert.match(await tooltip.innerText(), /Infantry/);
    assert.match(await tooltip.innerText(), /Cavalry/);
    assert.doesNotMatch(await tooltip.innerText(), /E4|#\d/);
    assert.equal(await ships.getAttribute('title'), null);
    await page.screenshot({ path: `/tmp/clash-tile-hover-ship-${width}.png` });
    await page.keyboard.press('Escape');
    await tooltip.waitFor({ state: 'hidden' });
    await page.evaluate(() => {
      prefs.colorBlind = true;
      host.emit('preferences', prefs);
    });
    await page.waitForTimeout(300);
    assert.equal(await tooltip.isVisible(), false, 'A dismissed tooltip stays hidden across state updates');
    await page.getByRole('button', { name: 'Strategy map', exact: true }).click();
    const tile = page.locator('.strategy-map-tile[data-position="D2"]');
    await tile.hover();
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /2\s*Infantry/);
    assert.match(await tooltip.innerText(), /1\s*Elephant/);
    const tooltipRect = await tooltip.boundingBox(),
      boardRect = await page.locator('.map-world').boundingBox();
    assert.ok(
      tooltipRect.x >= boardRect.x && tooltipRect.x + tooltipRect.width <= boardRect.x + boardRect.width + 1,
    );
    assert.ok(
      tooltipRect.y >= boardRect.y &&
        tooltipRect.y + tooltipRect.height <= boardRect.y + boardRect.height + 1,
    );
    await page.screenshot({ path: `/tmp/clash-tile-hover-army-${width}.png` });
    // Hover the canvas beneath the Strategy overlay: no badge is needed to inspect a tile.
    const box = await tile.boundingBox();
    await page.mouse.move(0, 0);
    await tooltip.waitFor({ state: 'hidden' });
    await page.addStyleTag({ content: '.strategy-map-tile{pointer-events:none!important}' });
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /2\s*Infantry/);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2);
    await tooltip.waitFor({ state: 'hidden' });
    await page.mouse.up();
    await page.mouse.move(0, 0);
    await page.addStyleTag({ content: '.strategy-map-tile{pointer-events:auto!important}' });
    const pirateTile = page.locator('.strategy-map-tile[data-position="F6"]');
    await pirateTile.hover();
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /2\s*Pirate ship/);
    const changed = JSON.parse(raw);
    changed.players.find((p) => p.civilization === 'Pirates').units.pop();
    await page.evaluate((raw) => host.emit('state', raw), JSON.stringify(changed));
    await page.locator('.strategy-map-tile[data-position="F6"]').hover();
    await page.waitForFunction(() =>
      /1\s*Pirate ship/.test(document.querySelector('.tile-hover').textContent),
    );
    assert.doesNotMatch(await tooltip.innerText(), /2\s*Pirate ship/);
    await page.keyboard.press('Escape');
    await tooltip.waitFor({ state: 'hidden' });
    await page.keyboard.press('Tab');
    await page.locator('.strategy-map-tile[data-position="F6"]').focus();
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /Pirate ship/);
    await page
      .locator('.strategy-map-tile[data-position="F6"]')
      .dispatchEvent('pointerenter', { pointerType: 'touch' });
    await page.locator('.map-world').dispatchEvent('pointermove', { pointerType: 'touch' });
    await tooltip.waitFor({ state: 'hidden' });
    await page.getByRole('button', { name: 'Strategy map', exact: true }).click();
    await page.getByRole('button', { name: 'Move units and found cities', exact: true }).click();
    const movementTile = page.locator('.map-hit-target[data-position="D2"]');
    await page.keyboard.press('Tab');
    await movementTile.focus();
    await tooltip.waitFor({ state: 'visible' });
    assert.match(await tooltip.innerText(), /2\s*Infantry/);
    await movementTile.evaluate((node) => node.blur());
    await page.mouse.move(0, 0);
    await tooltip.waitFor({ state: 'hidden' });
    await movementTile.dispatchEvent('pointerenter', { pointerType: 'touch' });
    await page.waitForTimeout(300);
    assert.equal(await tooltip.isVisible(), false, 'Touch selection never opens a hover tooltip');
    assert.deepEqual(sent, []);
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log(
    'Tile hover: 3D/Strategy, canvas and keyboard, passengers, pirates, refresh, drag/touch dismissal, and viewport bounds passed.',
  );
} finally {
  await browser.close();
}
