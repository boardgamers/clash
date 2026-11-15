import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

// Offline fixture only; optionally verify downloaded production viewer assets.
const engine = createRequire(import.meta.url)('../.engine/server.js');
let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'foreign-owner-hover', {});
for (const civ of ['Rome', 'Greece'])
  raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(raw));
const game = JSON.parse(raw);
game.map.tiles = [];
for (const col of ['C', 'D', 'E'])
  for (let row = 3; row <= 5; row++) game.map.tiles.push([col + row, col === 'E' ? 'Water' : 'Fertile']);
for (const player of game.players) {
  player.cities = [];
  player.units = [];
}
game.players[0].civilization = 'Rome';
game.players[1].civilization = 'Greece';
game.players[0].cities = [
  {
    position: 'D4',
    mood_state: 'Happy',
    city_pieces: { academy: 1, fortress: 0, port: 1, temple: 1 },
    port_position: 'E4',
  },
  { position: 'C3', mood_state: 'Happy', city_pieces: { temple: 0 } },
];
game.players[0].units = Array.from({ length: 4 }, (_, id) => ({ id, unit_type: 'Ship', position: 'E4' }));
raw = engine.stripSecret(JSON.stringify(game), 0);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [],
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
      if (!/^\/(viewer.js|server_bg.wasm|[a-z-]+.json)$/.test(path)) return route.abort();
      const body = process.env.VIEWER_URL
        ? Buffer.from(await (await fetch(new URL(path.slice(1), process.env.VIEWER_URL))).arrayBuffer())
        : await readFile(new URL('../dist' + path, import.meta.url));
      return route.fulfill({
        contentType: path.endsWith('.wasm')
          ? 'application/wasm'
          : path.endsWith('.json')
            ? 'application/json'
            : 'text/javascript',
        body,
      });
    });
    await page.goto('http://clash.test/'); // No prototype switches.
    await page.evaluate((raw) => {
      window.host = clash3d.launch('#app');
      window.prefs = { sound: false };
      host.on('move', (a) => window.moveSent(a));
      host.on('update:preference', ({ name, value }) => {
        prefs[name] = value;
        host.emit('preferences', prefs);
      });
      host.emit('player', { index: 1 }); // Foreign relative to city, not viewer.
      host.emit('preferences', prefs);
      host.emit('state', raw);
    }, raw);
    for (const strategy of [false, true]) {
      if (strategy) await page.getByRole('button', { name: 'Strategy map', exact: true }).click();
      const tile = strategy
        ? page.locator('.strategy-map-tile[data-position="D4"]')
        : page.getByRole('button', { name: /Select Rome city D4/ });
      await page.keyboard.press('Tab');
      await tile.focus();
      const tooltip = page.locator('.tile-hover:visible');
      await tooltip.waitFor();
      const text = await tooltip.innerText();
      for (const name of ['Academy', 'Port', 'Temple'])
        assert.match(text, new RegExp(`${name}\\s*·\\s*Greece`));
      assert.match(text, /Rome city.*D4/s);
      assert.match(text, /Happy/);
      assert.doesNotMatch(text, /Fortress\s*·/);
      assert.doesNotMatch(await tile.innerText(), /Greece/, 'No on-board civilization labels');
      await page.screenshot({ path: `/tmp/clash-owner-hover-${strategy ? 'strategy' : '3d'}-${width}.png` });
      await page.evaluate(() => {
        prefs.locale = 'de';
        host.emit('preferences', prefs);
      });
      await page.waitForFunction(() =>
        /Griechenland/.test(document.querySelector('.tile-hover')?.textContent ?? ''),
      );
      assert.match(await tooltip.innerText(), /Tempel\s*·\s*Griechenland/);
      await page.evaluate(() => {
        prefs.locale = 'en';
        host.emit('preferences', prefs);
      });
      await page.waitForFunction(() =>
        /Temple/.test(document.querySelector('.tile-hover')?.textContent ?? ''),
      );
      await page.keyboard.press('Escape');
      const control = strategy
        ? page.locator('.strategy-map-tile[data-position="C3"]')
        : page.getByRole('button', { name: /Select Rome city C3/ });
      await page.keyboard.press('Tab');
      await control.focus();
      await tooltip.waitFor();
      assert.match(await tooltip.innerText(), /Temple/);
      assert.doesNotMatch(await tooltip.innerText(), /Temple\s*·|Greece/);
      await page.keyboard.press('Escape');
    }
    assert.deepEqual(errors, []);
    assert.deepEqual(sent, [], 'Inspection must not send gameplay moves');
    await page.close();
  }
  console.log(
    'Foreign building hover: default production viewer, 3D/Strategy, desktop/mobile keyboard accessibility, foreign Temple/Academy/Port and same-owner control passed.',
  );
} finally {
  await browser.close();
}
