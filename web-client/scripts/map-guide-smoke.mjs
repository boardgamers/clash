import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const initial = await engine.init(2, [], { civilization: 'Random' }, 'map-guide-smoke', {});
const seat = engine.currentPlayer(initial);
const game = JSON.parse(engine.stripSecret(initial, seat));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 780],
    [320, 640],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [],
      sent = [];
    let state = initial;
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = engine.tryMove(state, action, seat);
      await page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(state, seat));
    });
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
    await page.evaluate(
      ({ seat, state }) => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false, unitBadges: true });
        host.emit('state', state);
      },
      { seat, state: JSON.stringify(game) },
    );
    for (const name of ['Show regions', 'Show trade routes', 'Show pirate spawn positions', 'Show barbarian spawn and movement']) {
      const button = page.getByRole('button', {name, exact:true});
      await button.click();
      const guide = page.locator('.collapsible-map-guide');
      await guide.waitFor();
      const expanded = (await guide.boundingBox()).height;
      await guide.locator('summary').click();
      assert.equal(await guide.getAttribute('open'), null);
      const collapsed = (await guide.boundingBox()).height;
      assert(collapsed < expanded, `${name}: collapse reduces height`);
      assert.equal(await button.getAttribute('aria-pressed'), 'true');
      await guide.locator('summary').press('Enter');
      assert.notEqual(await guide.getAttribute('open'), null);
      await button.click();
    }
    assert.deepEqual(errors, []);
    console.log(`${width}px: all four guides collapse, retain overlays and expand by keyboard`);
    await page.close();
  }
} finally { await browser.close(); }
