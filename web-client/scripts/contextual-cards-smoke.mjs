import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const g = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/advances/collect_free_economy.json', import.meta.url),
    'utf8',
  ),
);
for (const p of JSON.parse(await engine.init(2, [], {}, 'context-card-ui', {})).players.slice(2))
  if (!g.players.some((q) => q.civilization === p.civilization))
    g.players.push({ ...p, id: g.players.length });
g.players[0].action_cards = [29, 19];
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 850 } }),
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
    await page.goto('http://clash.test/');
    await page.evaluate(
      (raw) => {
        window.sent = [];
        window.host = clash3d.launch('#app');
        host.on('move', (move) => sent.push(move));
        host.emit('player', { index: 0 });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', raw);
      },
      engine.stripSecret(JSON.stringify(g), 0),
    );
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    await page.getByRole('button', { name: /^Free Economy/ }).click();
    const list = page.locator('.collection-tile-list');
    await list.locator('summary').click();
    await list.getByRole('button').first().click();
    await list.locator('summary').click();
    const tilesBefore = await list.locator('summary').innerText();
    assert.match(tilesBefore, /Tiles 1/);
    const play = page.getByRole('button', { name: /Play Mass Production/ });
    assert.ok((await play.boundingBox()).height <= 44);
    await page.screenshot({ path: `/tmp/clash-contextual-cards-${width}.png` });
    await play.click();
    const sent = await page.evaluate(() => sent);
    assert.equal(sent.length, 1);
    assert.deepEqual(JSON.parse(sent[0]), { Playing: { ActionCard: 29 } });
    const next = engine.tryMove(JSON.stringify(g), sent[0], 0);
    await page.evaluate(
      (raw) => {
        host.emit('player', { index: 0 });
        host.emit('state', raw);
      },
      engine.stripSecret(next, 0),
    );
    await page.getByText('Mass Production · +2 tiles', { exact: true }).waitFor();
    assert.match(await list.locator('summary').innerText(), /Tiles 1/);
    assert.equal(
      await page.getByRole('button', { name: /^Free Economy/ }).getAttribute('aria-pressed'),
      'true',
    );
    assert.equal(await page.getByRole('button', { name: /Play Production Focus/ }).count(), 0);
    assert.equal(await page.locator('.collection-confirm > button').isEnabled(), true);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/clash-contextual-active-${width}.png` });
    console.log(
      `${width}: compact card shortcuts, selection and Free Economy preserved, boost active, no incompatible cards or overflow.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
