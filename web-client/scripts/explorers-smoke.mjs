import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)(process.env.EXPLORERS_ENGINE ?? '../.engine/server.js');
const dist = process.env.EXPLORERS_DIST ? new URL(`file://${process.env.EXPLORERS_DIST}/`) : new URL('../dist/', import.meta.url);
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const width of [1400, 390]) for (const civ of ['Japan', 'Rome']) for (const id of [21, 22]) {
    let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'explorers-ui', {});
    for (const choice of [civ, civ === 'Rome' ? 'India' : 'Rome'])
      raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: choice }), engine.currentPlayer(raw));
    let state = JSON.parse(raw);
    const seat = engine.currentPlayer(raw), p = state.players[seat];
    p.action_cards = [id];
    p.resources.culture_tokens = 3;
    state.map.unexplored_blocks = [];
    state.map.tiles = state.map.tiles.map(([pos, terrain]) => [pos, typeof terrain === 'object' || terrain === 'Unexplored' ? 'Fertile' : terrain]);
    const before = structuredClone(state), city = p.cities[0].position;
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [], sent = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.exposeFunction('applyMove', async action => {
      const parsed = typeof action === 'string' ? JSON.parse(action) : action;
      state = JSON.parse(engine.tryMove(JSON.stringify(state), JSON.stringify(parsed), seat));
      sent.push(parsed);
      await page.evaluate(state => host.emit('state', state), engine.stripSecret(JSON.stringify(state), seat));
    });
    await page.route('**/*', async route => {
      const path = new URL(route.request().url()).pathname;
      if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
      if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
      return route.fulfill({ contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body: await readFile(new URL(path.slice(1), dist)) });
    });
    await page.goto('http://clash.test/');
    await page.evaluate(({ state, seat }) => {
      window.host = clash3d.launch('#app');
      host.on('move', action => window.applyMove(action));
      host.emit('player', { index: seat });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    }, { state: engine.stripSecret(JSON.stringify(state), seat), seat });
    await page.getByRole('button', { name: 'Action cards: 1', exact: true }).click();
    await page.getByRole('button', { name: 'Play Explorers', exact: true }).click();
    const panel = page.locator('.decision-panel');
    await panel.getByText('Gain a free settler', { exact: false }).waitFor();
    assert.equal(await page.getByText('Choose a region to explore', { exact: false }).count(), 0);
    assert.deepEqual(await page.locator('.map-hit-target').evaluateAll(els => els.map(e => e.dataset.position)), [city]);
    const target = page.locator(`.map-hit-target[data-position="${city}"]`);
    await page.waitForTimeout(400);
    const box = await target.boundingBox();
    assert.ok(box);
    // The canvas handles map pointer events; labels provide the projected position.
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await page.screenshot({ path: `/tmp/clash-explorers-${civ}-${id}-${width}.png` });
    await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
    await panel.waitFor({ state: 'detached' });
    assert.deepEqual(sent, [{ Playing: { ActionCard: id } }, { Response: { Payment: [{ culture_tokens: 1 }] } }, { Response: { SelectPositions: [city] } }]);
    const after = state.players[seat];
    assert.equal(after.units.length, p.units.length + 1);
    assert.equal(after.units.at(-1).unit_type, 'Settler');
    assert.equal(after.units.at(-1).position, city);
    assert.deepEqual(after.cities, p.cities);
    assert.equal(after.resources.culture_tokens, 2);
    assert.equal(state.actions_left, before.actions_left - 1);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    console.log(`${width}px ${civ} A${id}: fully revealed map → card play → free settler; one action/culture, no activation or exploration prompt`);
    await page.close();
  }
} finally {
  await browser.close();
}
