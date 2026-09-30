import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'construction-smoke', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const move = (game, action) =>
  JSON.parse(
    engine.tryMove(JSON.stringify(game), typeof action === 'string' ? action : JSON.stringify(action), 0),
  );
let engineer = await fixture('incidents/great_persons/great_engineer');
for (const action of [
  { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } },
  { Response: { Payment: [{ culture_tokens: 1 }] } },
  { Playing: { ActionCard: 126 } },
  { Response: { SelectAdvance: 'Engineering' } },
  { Response: { Bool: true } },
])
  engineer = move(engineer, action);
let development = await fixture('action_cards/city_development');
development.players[0].cities[0].activations = 1;
development = move(move(development, { Playing: { ActionCard: 17 } }), {
  Response: { Payment: [{ culture_tokens: 1 }] },
});
const ordinary = await fixture('action_cards/city_development');
ordinary.players[0].cities[0].activations = 1;
ordinary.players[0].resources = { food: 3, wood: 3, ore: 3 };
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height, dark] of [
    [1400, 900, false],
    [390, 844, true],
    [320, 640, false],
  ]) {
    for (const [name, initial, label, resources, activation] of [
      ['engineer', engineer, 'Great Engineer · No extra action · No city activation', true, false],
      ['development', development, 'City Development · No extra action · Activates this city', false, true],
      ['ordinary', ordinary, 'Costs 1 action · Activates this city', true, true],
    ]) {
      const page = await browser.newPage({ viewport: { width, height } }),
        errors = [];
      let state = structuredClone(initial),
        sent = 0;
      page.on('pageerror', (error) => errors.push(error.message));
      await page.exposeFunction('applyMove', async (action) => {
        state = move(state, action);
        sent++;
        await page.evaluate(
          (state) => host.emit('state', state),
          engine.stripSecret(JSON.stringify(state), 0),
        );
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
        ({ state, dark }) => {
          window.host = clash3d.launch('#app');
          host.on('move', (action) => window.applyMove(action));
          host.emit('player', { index: 0 });
          host.emit('theme', { dark });
          host.emit('preferences', { sound: false });
          host.emit('state', state);
        },
        { state: engine.stripSecret(JSON.stringify(state), 0), dark },
      );
      await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
      await page
        .locator('.building-option')
        .filter({ has: page.getByText('Fortress', { exact: true }) })
        .click();
      const footer = page.locator('.city-confirm');
      await footer.getByText(label, { exact: true }).waitFor();
      assert.equal(await footer.locator('.resource-amount svg').count(), resources ? 3 : 0);
      assert.equal(await page.locator('.activation-status').count(), activation ? 1 : 0);
      if (activation) await page.getByText('This activation lowers mood.', { exact: false }).waitFor();
      assert.ok(await footer.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      assert.ok(await page.locator('.city-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.screenshot({ path: `/tmp/clash-build-${name}-${width}.png` });
      await page.getByRole('button', { name: 'Build Fortress', exact: true }).click();
      await page.waitForFunction(() => !document.querySelector('.city-confirm'));
      assert.equal(sent, 1);
      assert.equal(state.players[0].cities[0].city_pieces.fortress, 0);
      assert.equal(state.players[0].cities[0].mood_state, activation ? 'Angry' : 'Neutral');
      assert.deepEqual(errors, []);
      console.log(`${width}px ${name}: footer costs, activation warning and successful build verified.`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
