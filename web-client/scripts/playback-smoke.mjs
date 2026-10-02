import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let state = await engine.init(2, [], { civilization: 'Random' }, 'recap-browser', {});
const seat = engine.currentPlayer(state);
state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), seat);
const opponent = engine.currentPlayer(state),
  g = JSON.parse(state);
g.players[opponent].resources = { food: 7, ideas: 7, gold: 7, wood: 7, ore: 7 };
state = JSON.stringify(g);
const view = JSON.parse(engine.webView(engine.stripSecret(state, opponent), opponent));
state = engine.tryMove(state, JSON.stringify(view.advances.find((a) => a.id === 'Writing').action), opponent);
const publicState = engine.stripSecret(state, seat),
  history = JSON.parse(publicState).board_history;
assert.ok(history.frames.at(-1).effects?.some((e) => e.kind === 'action'));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
async function setup(page) {
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
}
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 850 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await setup(page);
    await page.evaluate(
      ({ raw, seat }) => {
        window.sent = [];
        window.replayInfo = [];
        window.host = clash3d.launch('#app');
        host.on('move', (m) => sent.push(m));
        host.on('replay:info', (info) => replayInfo.push(info));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', raw);
      },
      { raw: publicState, seat },
    );
    const bar = page.getByRole('region', { name: 'Board replay' });
    await bar.waitFor();
    assert.match(await bar.innerText(), /Since your last turn/);
    assert.equal(await page.locator('.board-toolbar').isVisible(), false);
    await page.getByText('Action card drawn', { exact: true }).waitFor();
    assert.equal(await page.locator('.public-effect .card-detail').count(), 0);
    await page.screenshot({ path: `/tmp/clash-replay-${width}.png` });
    await page.getByRole('button', { name: 'Skip', exact: true }).click();
    await bar.waitFor({ state: 'detached' });
    assert.equal(await page.locator('.board-toolbar').isVisible(), true);
    assert.deepEqual(await page.evaluate(() => sent), []);
    await page.evaluate(() => host.emit('replay:start'));
    await bar.waitFor();
    assert.match(await bar.innerText(), /Replay/);
    await page.evaluate((cursor) => host.emit('replay:to', cursor), history.frames.at(-1).cursor);
    await page.waitForFunction(
      (cursor) => replayInfo.at(-1)?.current === cursor,
      history.frames.at(-1).cursor,
    );
    await page.getByRole('button', { name: 'Done', exact: true }).click();
    await bar.waitFor({ state: 'detached' });
    await page.evaluate(() => host.emit('preferences', { analysis: true, sound: false }));
    await page.evaluate(() => host.emit('replay:start'));
    assert.equal(await bar.count(), 0);
    await page.getByText('Analysis · Simulation', { exact: true }).waitFor();
    assert.deepEqual(errors, []);
    console.log(
      `${width}: automatic catch-up, card back, Skip, native replay seek, analysis isolation; no moves submitted.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
