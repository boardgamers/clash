// Offline built-viewer regression; no BGS connections or live moves.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const raw = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/incidents/pandemics/drought.json', import.meta.url),
    'utf8',
  ),
);
raw.players[0].resources.food = 1;
const after = engine.tryMove(
  JSON.stringify(raw),
  JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
  0,
);
assert.equal(JSON.parse(after).players[0].resources.food ?? 0, 0);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
  ]) {
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
      (state) => {
        window.host = clash3d.launch('#app');
        host.emit('player', { index: 0 });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      engine.stripSecret(after, 0),
    );
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    const event = page.locator('.journal-event').filter({ hasText: /Drought|Draught/ });
    await event.waitFor();
    console.log(width, await event.innerText());
    const food = event.locator('.journal-delta').filter({ has: page.locator('b', { hasText: '−1' }) });
    assert.equal(await food.count(), 1);
    assert.match(await food.getAttribute('aria-label'), /food/i);
    await event.locator('.event-rules summary').click();
    const rules = JSON.parse(engine.webView(engine.stripSecret(after))).eventCatalog.find(
      (e) => e.id === 52,
    ).rules;
    const renderedRules = (await event.locator('.event-rules').innerText()).replace(/\s+/g, ' ');
    for (const rule of rules) assert.ok(renderedRules.includes(rule), rule);
    console.log('Expanded rules verified:', rules);
    await page.screenshot({ path: `/tmp/famine-journal-${width}.png` });
    assert.deepEqual(errors, []);
    await page.close();
  }
} finally {
  await browser.close();
}
