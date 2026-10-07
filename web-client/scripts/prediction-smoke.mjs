import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
// Optimistic moves against a host that answers only when told to: results show
// before the server responds, and matching confirmations do not touch the page.
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'prediction-ui', {})).players.slice(2);
const game = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/advances/collect_free_economy.json', import.meta.url),
    'utf8',
  ),
);
for (const npc of npcs)
  if (!game.players.some((p) => p.civilization === npc.civilization))
    game.players.push({ ...npc, id: game.players.length });
const ore = {
  Playing: {
    Collect: {
      city_position: 'C2',
      collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
      action_type: 'Collect',
    },
  },
};
// A live game already has public playback history.
const frames = JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(ore), 0)).board_history.frames;
const initial = JSON.stringify({
  ...game,
  board_history: { id: 'prediction-smoke', frames: frames.slice(0, 1) },
});
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
  ]) {
    let state = initial;
    const queued = [],
      errors = [];
    const page = await browser.newPage({ viewport: { width, height } });
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('queueMove', (move) => queued.push(move));
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
    await page.goto('http://clash.test');
    await page.evaluate(
      (state) => {
        window.host = clash3d.launch('#app');
        window.ready = false;
        host.on('ready', () => (window.ready = true));
        host.on('move', (m) => window.queueMove(m));
        host.emit('player', { index: 0 });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      engine.stripSecret(state, 0),
    );
    await page.waitForFunction(() => window.ready);
    const banner = page.locator('.turn-banner');
    const undo = page.getByRole('button', { name: 'Undo last action', exact: true });
    const redo = page.getByRole('button', { name: 'Redo last undone action', exact: true });
    // Answer the oldest queued move, then count DOM mutations caused by it.
    const confirm = async () => {
      const move = queued.shift();
      assert(move, 'a move was sent');
      state = engine.tryMove(state, move, 0);
      const mutations = await page.evaluate(
        async (raw) => {
          let count = 0;
          const observer = new MutationObserver((records) => (count += records.length));
          observer.observe(document.querySelector('#app'), {
            subtree: true,
            childList: true,
            attributes: true,
            characterData: true,
          });
          host.emit('move:result', { move: '', ok: true });
          host.emit('state', raw);
          await new Promise((resolve) => setTimeout(resolve, 400));
          observer.disconnect();
          return count;
        },
        engine.stripSecret(state, 0),
      );
      assert.equal(mutations, 0, 'a confirmed prediction does not re-render');
    };

    assert(await undo.isDisabled());
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    await page.locator('.collect-map-label').first().click();
    await page.locator('.collect-submit').click();
    // No server answer yet: the result is already shown and actions stay available.
    await page.waitForFunction(() => !document.querySelector('.turn-banner.confirming'));
    assert(await undo.isEnabled());
    assert.doesNotMatch(await banner.innerText(), /Confirming/);
    assert.equal(queued.length, 1);
    await confirm();

    await undo.click();
    await redo.waitFor();
    assert(await undo.isDisabled());
    assert.equal(queued.length, 1);
    await confirm();

    await redo.click();
    await undo.waitFor({ state: 'visible' });
    assert(await undo.isEnabled());
    await confirm();

    // A rejection returns to the server's position immediately.
    await undo.click();
    await redo.waitFor();
    await page.evaluate(
      (move) => host.emit('move:result', { move, ok: false, error: 'Rejected' }),
      queued.shift(),
    );
    await page.waitForFunction(() => !document.querySelector('.board-toolbar.has-redo'));
    assert(await undo.isEnabled());
    await page.screenshot({ path: `/tmp/clash-prediction-${width}.png` });
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: collect, undo and redo show at once; confirmations cause no DOM change; rejection rolls back.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
