import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'wonder-payment-ui', {})).players.slice(2);
const original = JSON.parse(
  await readFile(new URL('../../server/tests/test_games/wonders/colosseum.json', import.meta.url), 'utf8'),
);
original.players.push(...npcs);
const move = (state, action) =>
  JSON.parse(
    engine.tryMove(JSON.stringify(state), typeof action === 'string' ? action : JSON.stringify(action), 0),
  );
const view = (state) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(state), 0), 0));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390, 320])
    for (const count of [4, 10, 34]) {
      let state = structuredClone(original),
        sent = 0,
        lastAction;
      state.players[0].resources =
        count === 4
          ? { food: 3, wood: 1, ore: 2, gold: 7, culture_tokens: 5 }
          : { food: 3, wood: 4, ore: 5, gold: count === 10 ? 2 : 4, culture_tokens: 5 };
      state = move(state, { Playing: { WonderCard: 'Colosseum' } });
      const options = view(state).decision.fields[0].choices;
      assert.equal(options.length, count);
      const resources = { ...state.players[0].resources };
      const page = await browser.newPage({ viewport: { width, height: width === 1400 ? 1000 : 740 } }),
        errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.exposeFunction('applyMove', async (action) => {
        lastAction = typeof action === 'string' ? JSON.parse(action) : action;
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
      await page.goto('http://clash.test');
      await page.evaluate(
        ({ state }) => {
          window.host = clash3d.launch('#app');
          host.on('move', (a) => window.applyMove(a));
          host.emit('player', { index: 0 });
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false });
          host.emit('state', state);
        },
        { state: engine.stripSecret(JSON.stringify(state), 0) },
      );
      const panel = page.locator('.decision-panel');
      await panel.waitFor();
      for (const name of ['Sound','Color-blind mode'])
        assert.equal(await page.locator(`.table-tools [aria-label="${name}"]`).isVisible(), width>760);
      assert.equal(await page.getByRole('button',{name:'Open journal',exact:true}).isVisible(),true);
      assert.equal(await page.getByRole('button',{name:'Open chat',exact:true}).isVisible(),true);
      if (count === 4) {
        assert.equal(await panel.locator('.compact-payments button').count(), 4);
        assert.equal(await panel.locator('select').count(), 0);
        assert(
          (await panel.locator('.compact-payments button').first().boundingBox()).height <= 48,
          'Direct payment buttons remain compact',
        );
      } else {
        const picker = panel.locator('.payment-builder');
        await picker.waitFor();
        assert.equal(await picker.locator('.payment-fixed [aria-label="5 Culture"]').count(), 1);
        assert.equal(await picker.locator('select').count(), 4);
        await picker.getByLabel(/Food amount/).selectOption(count === 10 ? '1' : '0');
        assert.equal(sent, 0, 'Choosing an amount does not submit');
      }
      assert(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.screenshot({ path: `/tmp/clash-wonder-payment-${count}-${width}.png` });
      if (count === 4) await panel.locator('.compact-payments button').first().click();
      else await panel.locator('.payment-builder-actions .primary').click();
      await page.waitForFunction(() => !document.querySelector('.decision-panel'));
      assert.equal(sent, 1, 'One click submits a payment once');
      const paid = lastAction.Response.Payment[0];
      assert(options.some((o) => JSON.stringify(o) === JSON.stringify(paid)));
      for (const [resource, amount] of Object.entries(paid))
        assert.equal(state.players[0].resources[resource] ?? 0, (resources[resource] ?? 0) - amount);
      assert(state.players[0].cities.some((c) => c.city_pieces?.wonders?.includes('Colosseum')));
      if (count === 10) {
        await page.locator('.city-map-label').first().click();
        await page.getByRole('button', { name: 'Colosseum', exact: true }).click();
        const info = page.getByRole('region', { name: 'Colosseum effect', exact: true });
        await info.waitFor();
        assert.match(await info.innerText(), /combat value/);
        assert(await info.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
        await page.screenshot({ path: `/tmp/clash-built-wonder-${width}.png` });
        await page.getByRole('button', { name: 'Close tile actions', exact: true }).click();
        const city = state.players[0].cities[0];
        city.city_pieces = {
          academy: 0,
          market: 0,
          temple: 0,
          port: 0,
          wonders: ['GreatLighthouse', 'Pyramids'],
        };
        await page.evaluate(
          (state) => host.emit('state', state),
          engine.stripSecret(JSON.stringify(state), 0),
        );
        await page.locator('.city-map-label').first().click();
        for (const name of ['Great Lighthouse', 'Pyramids']) {
          await page.getByRole('button', { name, exact: true }).click();
          const effect = page.getByRole('region', { name: `${name} effect`, exact: true });
          await effect.waitFor();
          if (name === 'Pyramids')
            assert.match(await effect.innerText(), /5\.1 VP for building · 0 VP for owning/);
        }
        await page.screenshot({ path: `/tmp/clash-multiple-wonders-${width}.png` });
        await page.getByRole('button', { name: 'Close tile actions', exact: true }).click();
        await page.getByRole('button', { name: 'Toggle top-down view', exact: true }).click();
        await page.screenshot({ path: `/tmp/clash-wonders-overhead-${width}.png` });
      }
      if (count === 34) {
        state = structuredClone(original);
        state.players[0].resources = { food: 4, ideas: 4, gold: 4, wood: 4, ore: 5, culture_tokens: 5 };
        await page.evaluate(
          (state) => host.emit('state', state),
          engine.stripSecret(JSON.stringify(state), 0),
        );
        await page.getByRole('button', { name: 'Research tree', exact: true }).click();
        await page.locator('#research-Arts .research-pick').click();
        const payment = page.getByRole('group', { name: 'Research payment', exact: true });
        await payment.getByLabel('Ideas amount for Research payment', { exact: true }).selectOption('2');
        assert(await page.locator('.research-detail').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
        await page.screenshot({ path: `/tmp/clash-research-payment-${width}.png` });
        await page.locator('.research-payment > button.primary').click();
        await page.waitForFunction(() => !document.querySelector('.research-dialog'));
        assert.equal(sent, 2);
        assert.deepEqual(lastAction.Playing.Advance.payment, { ideas: 2 });
        assert.equal(state.players[0].resources.ideas, 2);
      }
      if (count === 34) {
        state = JSON.parse(
          await readFile(
            new URL('../../server/tests/test_games/movement/movement.json', import.meta.url),
            'utf8',
          ),
        );
        state.players.push(...structuredClone(npcs));
        state.players[0].advances.push('Tactics');
        state.players[0].units = state.players[0].units.filter((u) => u.id === 0);
        state.players[1].cities[0].position = 'A3';
        state.players[1].units = [];
        state.map.tiles.find((t) => t[0] === 'C1')[1] = 'Mountain';
        await page.evaluate(
          (state) => host.emit('state', state),
          engine.stripSecret(JSON.stringify(state), 0),
        );
        const movement = page.getByRole('region', { name: 'Unit movement', exact: true });
        if (!(await movement.isVisible()))
          await page.getByRole('button', { name: 'Move units and found cities', exact: true }).click();
        const infantry = movement.getByRole('button', { name: 'Infantry', exact: true });
        if ((await infantry.getAttribute('aria-pressed')) !== 'true') await infantry.click();
        const hex = page.locator('.map-hit-target[data-position="C1"]');
        await hex.focus();
        await hex.press('Enter');
        const terrain = movement.getByRole('complementary', { name: 'Mountain rules', exact: true });
        await terrain.waitFor();
        assert.match(await terrain.innerText(), /cannot move again/);
        assert.match(await terrain.innerText(), /No terrain bonus to combat value/);
        assert(await movement.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
        assert((await terrain.boundingBox()).height < 115, 'Terrain explanation remains compact');
        await page.screenshot({ path: `/tmp/clash-terrain-move-${width}.png` });
        await movement.getByRole('button', { name: 'Move here', exact: true }).click();
        await page.waitForFunction(() => !document.querySelector('.settler-confirm'));
        assert.equal(sent, 3);
        assert(state.players[0].units[0].movement_restrictions.includes('Mountain'));
        const closeMovement = page.getByRole('button', { name: 'Close movement controls', exact: true });
        if (await closeMovement.isVisible()) await closeMovement.click();
        await page.locator('.city-map-label').nth(1).click();
        const forest = page.getByRole('complementary', { name: 'Forest rules', exact: true });
        await forest.waitFor();
        assert.match(await forest.innerText(), /later attack/);
        await page.screenshot({ path: `/tmp/clash-terrain-inspect-${width}.png` });
      }
      assert.deepEqual(errors, []);
      console.log(
        `${width}px / ${count} choices: compact payment, validated spend, constructed wonder${count === 10 ? ', clickable effects and multiple landmarks' : ''}.`,
      );
      await page.close();
    }
} finally {
  await browser.close();
}
