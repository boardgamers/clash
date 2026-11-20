// Offline built-viewer + real WASM regression; no production game writes.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, freeEconomy] of [
    [1400, false],
    [390, false],
    [1400, true],
    [390, true],
  ]) {
    let state = JSON.parse(
      await readFile(
        new URL('../../server/tests/test_games/advances/collect_free_economy.json', import.meta.url),
        'utf8',
      ),
    );
    const fresh = JSON.parse(await engine.init(2, [], {}, 'ballcourts-browser', {}));
    state.players.push(...fresh.players.slice(2));
    const player = state.players[0];
    player.civilization = 'Maya';
    player.advances = [
      'Farming',
      'Mining',
      'Arts',
      'Sports',
      ...(freeEconomy ? ['Voting', 'FreeEconomy'] : []),
    ];
    player.resources = { food: 5, wood: 5, ore: 5, mood_tokens: 0, culture_tokens: 3 };
    player.cities[0].mood_state = 'Neutral';
    player.cities[1].city_pieces.wonders = ['Colosseum'];
    const actions = state.actions_left;
    const sent = [],
      errors = [];
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, 0));
      await page.evaluate((s) => host.emit('state', s), engine.stripSecret(JSON.stringify(state), 0));
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
    await page.goto('http://clash.test');
    await page.evaluate(
      (s) => {
        window.host = clash3d.launch('#app');
        host.on('move', (a) => window.applyMove(a));
        host.emit('player', { index: 0 });
        host.emit('preferences', { sound: false });
        host.emit('state', s);
      },
      engine.stripSecret(JSON.stringify(state), 0),
    );
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    await page.locator('.ballcourts-toggle input').check();
    await page.locator('.collect-map-label').first().click();
    if (freeEconomy)
      await page
        .locator('.variant-picker')
        .getByRole('button', { name: /Free Economy/ })
        .click();
    await page.screenshot({ path: `/tmp/clash-ballcourts-before-${freeEconomy ? 'free-' : ''}${width}.png` });
    await page.locator('.collect-submit').click();
    if (freeEconomy) await page.getByRole('button', { name: 'Pay 1 culture', exact: true }).click();
    await page.waitForFunction(
      () => !document.querySelector('.board-collection') && !document.querySelector('.decision-panel'),
    );
    assert.equal(sent.length, freeEconomy ? 2 : 1, 'Only the pre-existing Free Economy payment response');
    if (freeEconomy) assert.deepEqual(sent[1], { Response: { Payment: [{ culture_tokens: 1 }] } });
    assert.equal(sent[0].Playing.Collect.ballcourts, true);
    assert.equal(state.players[0].resources.culture_tokens, freeEconomy ? 1 : 2);
    assert.equal(state.players[0].resources.mood_tokens ?? 0, 0);
    assert.equal(state.actions_left, actions - (freeEconomy ? 0 : 1));
    assert.equal(state.events?.length ?? 0, 0);
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/clash-ballcourts-after-${freeEconomy ? 'free-' : ''}${width}.png` });
    console.log(
      `${width}px: ${freeEconomy ? 'Free Economy + ' : ''}Ballcourts paid exactly ${freeEconomy ? 2 : 1} culture, no new decision.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
