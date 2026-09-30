import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const require = createRequire(import.meta.url),
  engine = require('../.engine/server.js');
const script = await readFile(new URL('../dist/viewer.js', import.meta.url), 'utf8');
assert(!script.includes('data:application/wasm'), 'Wasm must load once as an external asset');
const state = engine.stripSecret(
  await engine.init(2, [], { civilization: 'Random' }, 'bundle-test', {}),
  0,
);
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/') {
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.end(
        '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
      );
    } else {
      assert(/^\/(viewer\.js|server_bg\.wasm)$/.test(req.url));
      res.setHeader(
        'Content-Type',
        req.url.endsWith('wasm') ? 'application/wasm' : 'text/javascript; charset=utf-8',
      );
      res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
    }
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [390, 1400]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } }),
      errors = [],
      requests = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('request', (r) => requests.push(r.url()));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.evaluate((state) => {
      window.host = clash3d.launch('#app');
      window.readyCount = 0;
      host.on('ready', () => readyCount++);
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    }, state);
    await page.waitForFunction(() => readyCount === 1, {}, { timeout: 30000 });
    assert.deepEqual(errors, []);
    assert.equal(requests.filter((u) => u.endsWith('.wasm')).length, 1);
    assert(await page.locator('canvas').count());
    console.log(`${width}px: split WASM viewer ready, no errors`);
    await page.close();
  }
} finally {
  await browser.close();
  await new Promise((r) => server.close(r));
}
