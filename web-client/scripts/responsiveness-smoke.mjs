import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'abilities-ui', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
  ]) {
    const state = process.env.THEATERS_STATE
      ? JSON.parse(await readFile(process.env.THEATERS_STATE, 'utf8'))
      : await fixture('advances/sanitation_and_draft');
    const seat = process.env.THEATERS_STATE ? 1 : 0;
    if (!process.env.THEATERS_STATE) {
      state.players[0].advances.push('Arts', 'Theaters');
      state.players[0].resources.mood_tokens = 2;
      state.players[0].resources.culture_tokens = 2;
    }
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
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
      ({ state, seat }) => {
        window.host = clash3d.launch('#app');
        window.sent = [];
        host.on('move', (a) => sent.push(a));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: engine.stripSecret(JSON.stringify(state), seat), seat },
    );
    await page.getByRole('button', { name: 'Abilities and cultural influence', exact: true }).click();
    await page.evaluate(() => {
      window.boardReplacements = 0;
      window.clickTasks = [];
      new MutationObserver(records => {
        boardReplacements += records.flatMap(r => [...r.removedNodes]).filter(n => n.classList?.contains('city-map-label')).length;
      }).observe(document.querySelector('.world-labels'), { childList: true });
      new PerformanceObserver(list => clickTasks.push(...list.getEntries().map(e => e.duration))).observe({ type: 'longtask' });
    });
    const start = performance.now();
    await page.getByRole('button', { name: /Use Theaters/ }).click();
    const pay = page.getByRole('button', { name: /^Pay 1 Mood/i });
    await pay.waitFor();
    assert.equal(await pay.isEnabled(), true);
    const elapsed = performance.now() - start;
    assert.equal(await page.evaluate(() => sent.length), 1);
    await page.screenshot({ path: `/tmp/clash-responsive-theaters-${width}.png` });
    await pay.click();
    await page.waitForFunction(() => !document.querySelector('.decision-panel'));
    assert.equal(
      await page.evaluate(() => sent.length),
      1,
      'Conversion is queued and shown without a backend response',
    );
    await page.getByRole('button', { name: 'Undo last action', exact: true }).click();
    await page.screenshot({ path: `/tmp/clash-responsive-undo-${width}.png` });
    await pay.waitFor();
    assert.equal(await pay.isEnabled(), true, 'Undo restores the choice before the backend responds');
    assert.equal(await page.evaluate(() => sent.length), 1);

    const metrics = await page.evaluate(() => ({ boardReplacements, longTasks: clickTasks.map(Math.round) }));
    assert.equal(await page.locator('.journal').count(), 0, 'Closed journal does not render history');
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    await page.getByRole('tabpanel', { name: 'Journal', exact: true }).waitFor();
    assert.ok(await page.locator('.journal-round').count() > 0);
    await page.keyboard.press('Escape');
    assert.equal(await page.locator('.journal').count(), 0);
    assert.deepEqual(errors, []);
    console.log({ width, choiceReadyMs: Math.round(elapsed), serverAcknowledgements: 0, ...metrics });
    assert.equal(metrics.boardReplacements, 0, 'Resource-only actions and Undo must retain board models and labels');
    await page.close();
  }
  console.log('Responsiveness desktop/mobile smoke passed');
} finally {
  await browser.close();
}
