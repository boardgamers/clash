// Synthetic fixed dice only: never read or mutate a production game.
// CLASH_WASM and CLASH_VIEWER_DIR can point at downloaded release artifacts.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
const root = fileURLToPath(new URL('..', import.meta.url));
const engine = createRequire(import.meta.url)(process.env.CLASH_WASM || '../.engine/server.js');
const viewer = process.env.CLASH_VIEWER_DIR || path.join(root, 'dist');
let game = JSON.parse(await readFile(new URL('../../server/tests/test_games/combat/retreat.json', import.meta.url)));
const third = structuredClone(game.players[0]);
Object.assign(third, { id: 2, civilization: 'Persia', units: [], cities: [{ position: 'B2', mood_state: 'Happy' }], action_cards: [], objective_cards: [] });
game.players.splice(2, 0, third);
game.players.forEach((p, i) => p.id = i);
game.players[0].units = game.players[0].units.filter(u => u.id < 4).map(u => ({ ...u, unit_type: 'Infantry' }));
game.players[1].units = Array.from({ length: 4 }, (_, id) => ({ id, position: 'C1', unit_type: id === 3 ? 'Infantry' : 'Cavalry' }));
game.dice_roll_outcomes = Array(8).fill(4);
let raw = engine.move(JSON.stringify(game), JSON.stringify({ Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1', payment: {} } } }), 0);
assert.equal(engine.currentPlayer(raw), 1);
const clocks = engine.timeIncrements(raw);
raw = await engine.dropPlayer(raw, 1);
game = JSON.parse(raw);
assert.equal(game.players[1].units.length, 2);
assert.equal(engine.currentPlayer(raw), 0);
assert.equal(game.events.at(-1).handler.request.BoolRequest, 'Do you want to retreat?');
assert.deepEqual(engine.timeIncrements(raw), clocks);
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [], sent = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.exposeFunction('recordMove', action => sent.push(action));
    await page.route('**/*', async route => {
      const name = new URL(route.request().url()).pathname;
      if (name === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
      if (!['/viewer.js', '/server_bg.wasm'].includes(name)) return route.abort();
      return route.fulfill({ contentType: name.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body: await readFile(path.join(viewer, name)) });
    });
    await page.goto('http://clash.test/');
    await page.waitForFunction(() => typeof window.clash3d !== 'undefined');
    await page.evaluate(state => {
      window.host = clash3d.launch('#app');
      host.on('move', action => window.recordMove(action));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false, reducedMotion: true });
      host.emit('state', state);
    }, engine.stripSecret(raw, 0));
    const skip = page.getByRole('button', { name: 'Skip', exact: true });
    await skip.waitFor();
    await skip.click();
    await page.getByText('Do you want to retreat?', { exact: true }).waitFor();
    await page.getByRole('button', { name: /^Retreat$/i }).waitFor();
    await page.getByRole('button', { name: /^Continue battle$/i }).waitFor();
    await page.waitForTimeout(300);
    assert.deepEqual(sent, [], 'viewer must not invent any living-player action');
    assert.deepEqual(errors, []);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.close();
    console.log(`${width}px: dropped defender mandatory casualties resolved; living retreat visible, zero viewer moves.`);
  }
} finally {
  await browser.close();
}
