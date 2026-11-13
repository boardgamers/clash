// Offline built-viewer check: npm run build:bridge && npm run build && node scripts/piece-loss-journal-smoke.mjs
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = async (name) =>
  JSON.parse(await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
const move = (state, action) => engine.tryMove(state, JSON.stringify(action), 0);
const raw = await fixture('incidents/earthquake/earthquake');
raw.players[0].destroyed_structures = { cities: 2 };
let quake = move(JSON.stringify(raw), { Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } });
quake = move(quake, {
  Response: {
    SelectStructures: [
      { position: 'B2', structure: 'CityCenter' },
      { position: 'C2', structure: { Building: 'Temple' } },
    ],
  },
});
const raze = move(JSON.stringify(await fixture('combat/capture_and_raze_city')), {
  Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1', payment: {} } },
});
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 844],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/*', async (route) => {
      const url = new URL(route.request().url());
      if (url.hostname !== 'clash.test') return route.abort();
      if (url.pathname === '/')
        return route.fulfill({
          contentType: 'text/html',
          body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
        });
      if (!['/viewer.js', '/server_bg.wasm'].includes(url.pathname)) return route.abort();
      return route.fulfill({
        contentType: url.pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
        body: await readFile(new URL('../dist' + url.pathname, import.meta.url)),
      });
    });
    await page.goto('http://clash.test/');
    await page.evaluate(() => {
      window.host = clash3d.launch('#app');
      host.emit('player', {});
      host.emit('preferences', { sound: false });
    });
    for (const [name, state, messages] of [
      [
        'earthquake',
        quake,
        [
          'City center at B2 permanently destroyed; city limit reduced to 4',
          'Temple at C2 permanently destroyed; total piece limit reduced to 4',
        ],
      ],
      ['raze', raze, ['Razed C1 because no city centers were available']],
    ]) {
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(state));
      const toggle = page.getByRole('button', { name: 'Open journal', exact: true });
      await toggle.waitFor();
      if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click();
      for (const message of messages) {
        const note = page.locator('.journal').getByText(message, { exact: false });
        await note.first().waitFor();
        await note.first().scrollIntoViewIfNeeded();
      }
      assert.ok(await page.locator('.journal').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      await page.screenshot({ path: `/tmp/clash-piece-loss-${name}-${width}.png` });
    }
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: earthquake capacity, foreign building, and forced-raze journal notes visible without overflow`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
