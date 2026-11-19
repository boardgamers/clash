import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { exhaustedTerrainFixture } from './exhausted-terrain-fixture.mjs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const { state, seat, examples } = await exhaustedTerrainFixture(engine);
const raw = engine.stripSecret(state, seat);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 1000],
    [390, 780],
    [320, 640],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [],
      sent = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.exposeFunction('moveSent', (action) => sent.push(action));
    await page.route('**/*', async (route) => {
      const pathname = new URL(route.request().url()).pathname;
      if (pathname === '/')
        return route.fulfill({
          contentType: 'text/html',
          body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
        });
      if (!['/viewer.js', '/server_bg.wasm'].includes(pathname)) return route.abort();
      return route.fulfill({
        contentType: pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
        body: await readFile(new URL('../dist' + pathname, import.meta.url)),
      });
    });
    await page.goto('http://clash.test/');
    assert.deepEqual(errors, []);
    await page.evaluate(
      ({ raw, seat }) => {
        const host = clash3d.launch('#app');
        const preferences = { sound: false, mapView: '3d', homeAtBottom: false };
        host.on('move', (action) => window.moveSent(action));
        host.on('update:preference', ({ name, value }) => {
          preferences[name] = value;
          host.receive('preferences', preferences);
        });
        host.receive('player', { index: seat });
        host.receive('preferences', preferences);
        host.receive('theme', { dark: false });
        host.receive('state', raw);
      },
      { raw, seat },
    );
    const mode = page.getByRole('button', { name: 'Strategy map', exact: true });
    await mode.waitFor();
    await page.waitForTimeout(1500);
    await page.screenshot({ path: `/tmp/clash-exhausted-3d-${width}.png` });
    await mode.click();
    await page.keyboard.press('Tab');
    for (const example of examples) {
      const tile = page.locator(`.strategy-map-tile[data-position="${example.exhausted}"]`);
      await tile.waitFor();
      assert.equal(await tile.getAttribute('aria-label'), `Exhausted ${example.kind.toLowerCase()}`);
      const icon = tile.locator('.terrain-icon');
      assert.equal(
        await icon.getAttribute('width'),
        ['Forest', 'Mountain'].includes(example.kind) ? '30' : '18',
      );
      assert.equal(await icon.locator(':scope > path').count(), 1, 'exhaustion hatch remains visible');
      assert.equal(await tile.locator('svg.lucide').count(), 1, 'underlying terrain icon remains');
      // Keyboard focus inspects even map tiles visually behind floating mobile controls.
      await tile.focus();
      await page
        .locator('.tile-hover-terrain')
        .filter({ hasText: `Exhausted ${example.kind.toLowerCase()}` })
        .waitFor();
    }
    await page.mouse.move(0, 0);
    await page.screenshot({ path: `/tmp/clash-exhausted-strategy-${width}.png` });
    assert.deepEqual(errors, []);
    assert.deepEqual(sent, [], 'visual inspection must never submit moves');
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    console.log('Exhausted terrain 3D/Strategy viewer smoke passed', { width, examples });
    await page.close();
  }
} finally {
  await browser.close();
}
