// Real engine and ordinary viewer payment controls, synthetic Rome captures.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { chromium } from 'playwright';
const {engine, setup, capture, city} = createRequire(import.meta.url)('../../scripts/provinces-fixture.cjs');
const dist = process.env.CLASH_VIEWER_DIR || new URL('../dist/', import.meta.url).pathname;
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    for (const choice of ['unaffordable', 'decline', 'pay']) {
      let state = capture(await setup(choice === 'unaffordable' ? 0 : 1));
      assert.equal(city(state).mood_state, 'Neutral');
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [],
        moves = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.exposeFunction('applyMove', async (action) => {
        const parsed = typeof action === 'string' ? JSON.parse(action) : action;
        state = JSON.parse(engine.tryMove(JSON.stringify(state), JSON.stringify(parsed), 0));
        moves.push(parsed);
        await page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(JSON.stringify(state), 0));
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
          host.emit('player', { index: 0 });
          host.emit('preferences', { sound: false });
          host.emit('state', raw);
        },
        engine.stripSecret(JSON.stringify(state), 0),
      );
      await page.waitForTimeout(500);
      if (choice !== 'unaffordable') {
        const panel = page.locator('.decision-panel');
        await panel.waitFor();
        assert.match(await panel.innerText(), /Provinces/);
        await page.screenshot({path: `/tmp/clash-provinces-pending-${choice}-${width}.png`});
        if (choice === 'pay') await panel.getByRole('button', {name: /^Pay /}).click();
        else await panel.getByRole('button', {name: /Skip|Decline|No thanks|^No$|Continue/}).click();
        await page.waitForFunction(() => !document.querySelector('.decision-panel'));
        assert.equal(moves.length, 1);
      } else {
        assert.equal(await page.locator('.decision-panel').count(), 0);
        assert.equal(moves.length, 0);
      }
      assert.equal(city(state).mood_state, choice === 'pay' ? 'Happy' : 'Neutral');
      assert.equal(await page.getByRole('alert').count(), 0);
      assert.deepEqual(errors, []);
      await page.screenshot({path: `/tmp/clash-provinces-${choice}-${width}.png`});
      console.log(`${width}px Provinces ${choice}: real viewer and engine verified`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
