import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

// Synthetic real-engine earthquake: Greece's Temple in Rome's C2 city.
// No remote game is read or modified.
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = await readFile(
  new URL('../../server/tests/test_games/incidents/earthquake/earthquake.json', import.meta.url),
  'utf8',
);
const state = engine.tryMove(
  fixture,
  JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
  0,
);
const server = createServer(async (req, res) => {
  if (req.url === '/')
    return res.end(
      '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
    );
  if (!/^\/(viewer.js|server_bg.wasm|de.json)$/.test(req.url)) return res.writeHead(404).end();
  res.setHeader(
    'Content-Type',
    req.url.endsWith('.wasm')
      ? 'application/wasm'
      : req.url.endsWith('.json')
        ? 'application/json'
        : 'text/javascript',
  );
  res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1280, 390, 320]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      (state) => {
        window.host = clash3d.launch('#app');
        window.sent = [];
        host.on('move', (action) => sent.push(action));
        host.emit('player', { index: 0 });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      engine.stripSecret(state, 0),
    );
    await page.getByRole('button', { name: 'Dismiss event explanation', exact: true }).click();
    await page.getByRole('button', { name: 'C2', exact: true }).click();
    const temple = page.getByRole('button', { name: 'Temple Greece', exact: true });
    await temple.waitFor();
    assert.equal(await page.locator('.structure-owner').count(), 1);
    assert.equal(await page.getByRole('button', { name: 'Fortress', exact: true }).count(), 1);
    assert.equal(await page.getByRole('button', { name: 'City center', exact: true }).count(), 1);
    await temple.click();
    assert.equal(await temple.getAttribute('aria-pressed'), 'true');
    assert.equal(await page.evaluate(() => sent.length), 0, 'selection must not submit early');
    for (const locale of ['en', 'de']) {
      await page.evaluate((locale) => host.emit('preferences', { locale, sound: false }), locale);
      await page
        .getByRole('button', { name: locale === 'de' ? 'Tempel Griechenland' : 'Temple Greece', exact: true })
        .waitFor();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert.ok(
        await page
          .locator('.structure-owner')
          .evaluateAll((elements) => elements.every((el) => el.scrollWidth <= el.clientWidth)),
      );
      await page.screenshot({ path: `/tmp/clash-structure-owner-${locale}-${width}.png` });
      console.log(`Ownership label and accessible name: ${locale}, ${width}px`);
    }
    assert.deepEqual(errors, []);
    await page.close();
  }
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
