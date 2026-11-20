// Real production controller, viewer and WASM; only the platform transport is local.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const { battle, objectives, engine, move, planning } = createRequire(import.meta.url)('../../scripts/postcombat-queue-fixture.cjs');
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  const mobile = process.env.CLASH_MOBILE === '1';
  const page = await browser.newPage({ viewport: mobile ? { width: 390, height: 844 } : { width: 1400, height: 900 }, isMobile: mobile, hasTouch: mobile });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  let state = battle('win', false, game => {
    // The combat unit fixture omits NPCs; the full viewer expects both seats.
    game.players.push({ id: 2, civilization: 'Barbarians', next_unit_id: 0 },
      { id: 3, civilization: 'Pirates', next_unit_id: 0 });
  });
  const key = planning(state).key;
  state = move(state, { QueueMoves: { key, actions: [{ Movement: 'Stop' }], waiting: false } }, 0);
  const sent = [];
  const emit = () => page.evaluate(raw => host.emit('state', raw), engine.stripSecret(JSON.stringify(state), 0));
  await page.exposeFunction('applyMove', async action => {
    sent.push(JSON.parse(action));
    state = JSON.parse(engine.tryMove(JSON.stringify(state), action, 0));
    await emit();
  });
  await page.route('**/*', async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><div id="app"></div><script src="/viewer.js"></script>' });
    if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
    const hosted = process.env.CLASH_VIEWER_URL;
    const body = hosted
      ? Buffer.from(await (await fetch(new URL(path.slice(1), hosted))).arrayBuffer())
      : await readFile(new URL('../dist' + path, import.meta.url));
    return route.fulfill({ contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body });
  });
  await page.goto('http://clash.test/');
  assert.deepEqual(errors, []);
  await page.evaluate(() => {
    window.host = clash3d.launch('#app');
    host.on('move', action => window.applyMove(action));
    host.emit('player', { index: 0 });
    host.emit('theme', { dark: true });
    host.emit('preferences', { sound: false, reducedMotion: true });
  });
  await emit();
  await page.getByText('Placing a settler', { exact: true }).waitFor();
  assert.match(await page.locator('.concurrent-choice-row').innerText(), /1 queued/);
  state = objectives(state);
  await emit();
  await page.getByText('Choosing an objective', { exact: true }).waitFor();
  assert.match(await page.locator('.concurrent-choice-row').innerText(), /1 queued/);
  assert.equal(planning(state).key, key);
  await page.getByRole('button', { name: 'Wait for choice', exact: true }).click();
  await page.waitForFunction(() => !document.querySelector('.concurrent-choice-row'));
  assert.equal(sent.length, 1);
  assert.deepEqual(sent[0], { QueueMoves: { key, actions: [{ Movement: 'Stop' }], waiting: true } });
  assert.equal(state.state.Movement.movement_actions_left, 2);
  state = move(state, { Response: { SelectHandCards: [{ ObjectiveCard: 27 }] } }, 1);
  await emit();
  assert.equal(state.state, 'Playing');
  assert.equal(state.action_plan, undefined);
  await page.screenshot({ path: '/tmp/clash-postcombat-queue.png' });
  assert.deepEqual(errors, []);
  console.log('Real queue UI: settler → objective retains key/action; Wait submits private plan; defender claim then replays Stop.');
} finally {
  await browser.close();
}
