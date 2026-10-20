import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const state = engine.stripSecret(
  await engine.init(2, [], { civilization: 'Random' }, 'chinese-smoke', {}),
  0,
);
const viewerUrl = process.env.VIEWER_URL || 'https://boardgamers.space/__clash_assets__/viewer.js';
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [390, 1400]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [],
      failed = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('response', (r) => {
      if (r.status() >= 400) failed.push(`${r.status()} ${r.url()}`);
    });
    await page.route('https://boardgamers.space/__chinese_check__', (route) =>
      route.fulfill({
        contentType: 'text/html',
        body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div>',
      }),
    );
    if (!process.env.VIEWER_URL)
      await page.route('https://boardgamers.space/__clash_assets__/**', async (route) => {
        const path = new URL(route.request().url()).pathname.replace('/__clash_assets__/', '');
        assert(!path.includes('..'));
        const types = {
          json: 'application/json',
          wasm: 'application/wasm',
          woff2: 'font/woff2',
          js: 'text/javascript',
          css: 'text/css',
        };
        await route.fulfill({
          contentType: types[path.split('.').pop()] || 'application/octet-stream',
          body: await readFile(new URL('../dist/' + path, import.meta.url)),
        });
      });
    await page.goto('https://boardgamers.space/__chinese_check__');
    await page.addScriptTag({ url: viewerUrl });
    await page.evaluate((state) => {
      window.host = clash3d.launch('#app');
      window.readyCount = 0;
      host.on('ready', () => readyCount++);
      host.emit('preferences', { sound: false, locale: 'zh-CN' });
      host.emit('player', { index: 0 });
      host.emit('state', state);
    }, state);
    await page.waitForFunction(() => readyCount === 1);
    await page.waitForFunction(
      () =>
        document.querySelector('[data-clash-locale="zh-CN"]') &&
        /[\u4e00-\u9fff]/.test(document.body.innerText),
    );
    await page.evaluate(() => document.fonts.ready);
    assert(
      await page.evaluate(() => document.fonts.check('16px "Clash ChineseSimplified"', '简体中文城市进步')),
    );
    assert(await page.locator('canvas').count());
    const geometry = await page.evaluate(() => ({
      width: innerWidth,
      scroll: document.documentElement.scrollWidth,
    }));
    assert(geometry.scroll <= geometry.width + 1, JSON.stringify(geometry));
    await page.screenshot({ path: `/tmp/clash-chinese-${width}.png` });
    await page.evaluate(() => host.emit('preferences', { sound: false, locale: 'en' }));
    await page.waitForFunction(() => document.querySelector('[data-clash-locale="en"]'));
    await page.evaluate(() => host.emit('preferences', { sound: false, locale: 'zh-Hans-SG' }));
    await page.waitForFunction(
      () =>
        document.querySelector('[data-clash-locale="zh-CN"]') &&
        /[\u4e00-\u9fff]/.test(document.body.innerText),
    );
    assert.deepEqual(errors, []);
    assert.deepEqual(failed, []);
    console.log(`${width}px: Simplified Chinese cold load, font, layout and language switching passed`);
    await page.close();
  }
} finally {
  await browser.close();
}
