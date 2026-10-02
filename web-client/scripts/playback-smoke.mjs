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
const recruitment = JSON.parse(
  engine.webQuery(
    state,
    opponent,
    JSON.stringify({
      kind: 'recruit',
      city: g.players[opponent].cities[0].position,
      units: { infantry: 2 },
      replaced: [],
    }),
  ),
);
state = engine.tryMove(state, JSON.stringify(recruitment.action), opponent);
state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), opponent);
const publicState = engine.stripSecret(state, seat),
  history = JSON.parse(publicState).board_history;
assert.ok(history.frames.some((f) => f.effects?.some((e) => e.kind === 'action')));
const updated = engine.stripSecret(engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), seat), seat);
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
async function launch(page, unseen = false) {
  await page.evaluate(
    ({ raw, seat, unseen }) => {
      if (unseen)
        for (const key of Object.keys(localStorage))
          if (key.startsWith('clash:board-seen:')) localStorage.removeItem(key);
      window.sent = [];
      window.replayInfo = [];
      window.host = clash3d.launch('#app');
      host.on('move', (m) => sent.push(m));
      host.on('replay:info', (info) => replayInfo.push(info));
      let prefs = JSON.parse(localStorage.getItem('test-preferences') ?? '{"sound":false}');
      host.on('update:preference', ({ name, value }) => {
        prefs = { ...prefs, [name]: value };
        localStorage.setItem('test-preferences', JSON.stringify(prefs));
        host.emit('preferences', prefs);
      });
      host.emit('player', { index: seat });
      host.emit('theme', { dark: true });
      host.emit('preferences', prefs);
      host.emit('state', raw);
    },
    { raw: publicState, seat, unseen },
  );
}
try {
  for (const [width, height] of [
    [1400, 850],
    [390, 850],
    [320, 640],
    [844, 390],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await setup(page);
    await launch(page);
    const bar = page.getByRole('region', { name: 'Board replay' });
    const next = bar.getByRole('button', { name: 'Next action', exact: true });
    const back = bar.getByRole('button', { name: 'Previous action', exact: true });
    await bar.waitFor();
    assert.match(await bar.innerText(), /Since your last turn/);
    assert.equal(await page.locator('.board-toolbar').isVisible(), false);
    await bar.getByRole('button', { name: 'Pause replay', exact: true }).click();
    assert(await next.isVisible());
    assert(await back.isVisible());
    assert.equal(await bar.getByRole('checkbox', { name: 'Autoplay on return' }).isChecked(), false);
    await next.click();
    await page.getByText('Action card drawn', { exact: true }).waitFor();
    assert.equal(await page.locator('.public-effect .card-detail').count(), 0);
    await next.click();
    await page.getByText(/recruited 2 infantry/).waitFor();
    const caption = await bar.locator('.playback-description').innerText();
    await page.waitForTimeout(2700);
    assert.equal(
      await bar.locator('.playback-description').innerText(),
      caption,
      'manual step remains paused',
    );
    assert.match(caption, /Action 2 of 3/);
    const bounds = await bar.boundingBox();
    assert(bounds.x >= 0 && bounds.x + bounds.width <= width + 1 && bounds.height < 185);
    assert.equal(await bar.evaluate((el) => el.scrollWidth <= el.clientWidth), true);
    await page.screenshot({ path: `/tmp/clash-replay-${width}.png` });
    await next.click();
    await bar.getByRole('button', { name: 'Back to game', exact: true }).waitFor();
    assert(await next.isDisabled());
    await bar.getByRole('button', { name: 'Replay this turn', exact: true }).click();
    assert(await back.isDisabled());
    assert.match(await bar.innerText(), /Start/);
    await bar.getByRole('button', { name: 'Skip', exact: true }).click();
    const shortcut = page.getByRole('button', { name: 'Replay last turn', exact: true });
    await shortcut.waitFor();
    if (width <= 760) assert(await shortcut.getByText('Last turn', { exact: true }).isVisible());
    await shortcut.click();
    await bar.getByRole('button', { name: 'Play replay', exact: true }).waitFor();
    assert.match(await bar.innerText(), /Last turn/);
    assert(await back.isDisabled());
    await page.evaluate((raw) => host.emit('state', raw), updated);
    await next.click();
    assert.match(await bar.innerText(), /Action 1 of 3/, 'new live actions do not extend the selected turn');
    await bar.getByRole('button', { name: 'Skip', exact: true }).click();
    assert.deepEqual(await page.evaluate(() => sent), []);
    // Reopening with new activity uses the same saved manual preference.
    await page.reload();
    await launch(page, true);
    await bar.getByRole('button', { name: 'Play replay', exact: true }).waitFor();
    assert.equal(await bar.getByRole('checkbox', { name: 'Autoplay on return' }).isChecked(), false);
    await bar.getByRole('button', { name: 'Play replay', exact: true }).click();
    await bar.getByRole('button', { name: 'Back to game', exact: true }).waitFor({ timeout: 15000 });
    assert(await bar.isVisible(), 'autoplay finishes without dismissing the recap');
    await bar.getByRole('button', { name: 'Back to game', exact: true }).click();
    await page.evaluate(() => host.emit('replay:start'));
    await bar.waitFor();
    await page.evaluate((cursor) => host.emit('replay:to', cursor), history.frames.at(-1).cursor);
    await page.waitForFunction(
      (cursor) => replayInfo.at(-1)?.current === cursor,
      history.frames.at(-1).cursor,
    );
    await bar.getByRole('button', { name: 'Back to game', exact: true }).click();
    await page.evaluate(() => host.emit('preferences', { analysis: true, sound: false }));
    await page.evaluate(() => host.emit('replay:start'));
    assert.equal(await bar.count(), 0);
    assert.equal(await shortcut.count(), 0);
    await page.getByText('Analysis · Simulation', { exact: true }).waitFor();
    assert.deepEqual(errors, []);
    console.log(
      `${width}x${height}: manual/automatic recap, visible stepping, bounded last-turn replay, remembered preference, live updates, native replay and analysis isolation verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
