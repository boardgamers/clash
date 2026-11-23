// Synthetic legal cargo only. Production viewer/controller and actual WASM;
// no production saves, forced clicks, or private state repairs.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
const root = fileURLToPath(new URL('..', import.meta.url));
const engine = createRequire(import.meta.url)(process.env.CLASH_WASM || '../.engine/server.js');
const viewer = process.env.CLASH_VIEWER_DIR || path.join(root, 'dist');
function pending(template) {
  const game = structuredClone(template);
  [game.players[0], game.players[1]] = [game.players[1], game.players[0]];
  game.players.forEach((p, i) => (p.id = i));
  game.current_player_index = game.starting_player_index = 1;
  game.log[0].rounds[0].turns[0].turn_type.Player = 1;
  game.players[1].recruited_leaders = ['Ragnar'];
  game.players[1].units = [
    { id: 3, position: 'D2', unit_type: 'Ship' },
    { id: 4, position: 'D2', unit_type: { Leader: 'Ragnar' }, carrier_id: 3 },
    { id: 5, position: 'D2', unit_type: 'Infantry', carrier_id: 3 },
  ];
  return engine.move(
    JSON.stringify(game),
    JSON.stringify({ Movement: { Move: { units: [3], destination: 'C2', payment: {} } } }),
    1,
  );
}
const template = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/civilizations/vikings/ship_construction.json', import.meta.url),
  ),
);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390])
    for (const settler of [false, true]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [],
        sent = [];
      let raw = pending(template);
      page.on('pageerror', (error) => errors.push(error.message));
      const emit = () => page.evaluate((state) => host.emit('state', state), engine.stripSecret(raw, 1));
      await page.exposeFunction('applyMove', async (action) => {
        sent.push(JSON.parse(action));
        raw = engine.move(raw, action, 1);
        await emit();
      });
      await page.route('**/*', async (route) => {
        const name = new URL(route.request().url()).pathname;
        if (name === '/')
          return route.fulfill({
            contentType: 'text/html',
            body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
          });
        if (!['/viewer.js', '/server_bg.wasm'].includes(name)) return route.abort();
        return route.fulfill({
          contentType: name.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
          body: await readFile(path.join(viewer, name)),
        });
      });
      await page.goto('http://clash.test/');
      await page.waitForFunction(() => typeof window.clash3d !== 'undefined');
      await page.evaluate(() => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: 1 });
        host.emit('preferences', { sound: false, reducedMotion: true });
      });
      await emit();
      await page
        .getByText('Select units to convert to settlers (instead of infantry)', { exact: true })
        .waitFor();
      if (settler) {
        await page.getByRole('button', { name: 'Ship', exact: true }).click();
        await page.getByRole('button', { name: 'Confirm', exact: true }).click();
      } else {
        await page.getByRole('button', { name: 'Skip', exact: true }).click();
      }
      await page.waitForFunction(
        () => !document.body.textContent.includes('Select units to convert to settlers'),
      );
      assert.deepEqual(sent, [{ Response: { SelectUnits: settler ? [3] : [] } }]);
      const state = JSON.parse(raw);
      assert.deepEqual(
        state.players[1].units.map((u) => u.id),
        [3, 4, 5],
      );
      assert.equal(state.players[1].units[0].unit_type, settler ? 'Settler' : 'Infantry');
      assert.deepEqual(state.players[1].units[1].unit_type, { Leader: 'Ragnar' });
      assert.ok(state.players[1].units.every((u) => u.position === 'C2' && u.carrier_id == null));
      assert.equal(
        JSON.stringify(state.log).split('Unloaded 1 infantry and Ragnar Lodbrok from ships').length - 1,
        1,
      );
      assert.equal(state.state.Movement.movement_actions_left, 2);
      assert.deepEqual(state.state.Movement.moved_units, [3]);
      assert.ok(!(state.events || []).length);
      assert.doesNotThrow(() => engine.webView(raw, 1));
      assert.doesNotMatch(
        (await page.locator('.toast').allTextContents()).join(' '),
        /error|panic|invalid|failed/i,
      );
      assert.deepEqual(errors, []);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.close();
      console.log(
        `${width}px: normal Ship Construction ${settler ? 'Ship / Confirm → Settler' : 'Skip → Infantry'}, leader cargo unloaded once; valid resumed state, no error toast.`,
      );
    }
} finally {
  await browser.close();
}
