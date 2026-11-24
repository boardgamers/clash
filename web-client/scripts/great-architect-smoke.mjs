// Real engine, normal viewer clicks; only the frozen synthetic legal history is used.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)(process.env.CLASH_WASM || '../.engine/server.js');
const dist = process.env.CLASH_VIEWER_DIR || new URL('../dist/', import.meta.url).pathname;
const fixture = JSON.parse(
  await readFile(new URL('../../server/tests/fixtures/legacy_architect_170.json', import.meta.url)),
);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    for (const legacy of [false, true]) {
      let state = JSON.parse(
        engine.replay(JSON.stringify(fixture.history), { to: fixture[legacy ? 'pending' : 'before'] }),
      );
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [],
        moves = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.exposeFunction('applyMove', async (action) => {
        const parsed = typeof action === 'string' ? JSON.parse(action) : action;
        state = JSON.parse(engine.tryMove(JSON.stringify(state), JSON.stringify(parsed), 1));
        moves.push(parsed);
        await page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(JSON.stringify(state), 1));
      });
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname !== 'clash.test') return route.abort();
        if (url.pathname === '/')
          return route.fulfill({
            contentType: 'text/html',
            body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
          });
        return route.fulfill({
          contentType: url.pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
          body: await readFile(path.join(dist, url.pathname)),
        });
      });
      await page.goto('http://clash.test/');
      await page.evaluate(
        (raw) => {
          window.host = clash3d.launch('#app');
          host.on('move', (action) => window.applyMove(action));
          host.emit('player', { index: 1 });
          host.emit('preferences', { sound: false });
          host.emit('state', raw);
        },
        engine.stripSecret(JSON.stringify(state), 1),
      );
      if (!legacy) {
        await page.getByRole('button', { name: /^Wonders:/ }).click();
        await page.getByRole('button', { name: 'Use Great Architect', exact: true }).click();
      }
      const pay = page.locator('.decision-panel').getByRole('button', { name: /^Pay / });
      await pay.waitFor();
      assert.equal(state.actions_left, legacy ? 3 : 2);
      await pay.click();
      await page.waitForFunction(() => !document.querySelector('.decision-panel'));
      assert.equal(moves.length, legacy ? 1 : 2);
      assert.equal(state.actions_left, legacy ? 3 : 2);
      assert.ok(state.players[1].wonders_built.includes('GreatWall'));
      assert.ok(!state.players[1].action_cards.includes(155));
      assert.equal(await page.getByRole('alert').count(), 0);
      assert.deepEqual(errors, []);
      await page.screenshot({ path: `/tmp/clash-architect-${legacy ? 'legacy' : 'new'}-${width}.png` });
      console.log(
        `${width}px ${legacy ? 'legacy pending stays free' : 'new Architect costs one action'}: real payment and wonder verified`,
      );
      await page.close();
    }
  }
} finally {
  await browser.close();
}
