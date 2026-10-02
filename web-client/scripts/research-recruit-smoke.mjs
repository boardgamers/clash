import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let initial = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'research-recruit-copy', {});
for (const civ of ['China', 'Greece'])
  initial = engine.tryMove(
    initial,
    JSON.stringify({ ChooseCivilization: civ }),
    engine.currentPlayer(initial),
  );
initial = JSON.parse(initial);
const seat = engine.currentPlayer(JSON.stringify(initial));
const p = initial.players[seat];
p.advances = ['Farming', 'Mining', 'Storage', 'Sanitation', 'Draft', 'Tactics', 'Bartering'];
p.resources = { food: 0, wood: 7, ore: 7, mood_tokens: 7, culture_tokens: 0, gold: 0 };
p.cities[0].mood_state = 'Happy';
p.cities[0].city_pieces = { market: seat, academy: seat };
const city = p.cities[0].position;
const compact = (pile) => Object.fromEntries(Object.entries(pile).filter(([, n]) => n));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390, 320]) {
    let state = structuredClone(initial),
      sent = 0;
    const page = await browser.newPage({ viewport: { width, height: width === 1400 ? 1000 : 700 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, seat));
      sent++;
      await page.evaluate(
        (state) => host.emit('state', state),
        engine.stripSecret(JSON.stringify(state), seat),
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
      ({ state, seat }) => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: engine.stripSecret(JSON.stringify(state), seat), seat },
    );
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    await page.locator('.city-content').evaluate((el) => (el.scrollTop = 100));
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    assert.equal(await page.locator('.city-content').evaluate((el) => el.scrollTop), 0);
    const settler = page.getByRole('article', { name: 'Settler', exact: true });
    assert.match(await settler.innerText(), /Sanitation: 1 settler per recruitment/);
    assert.equal(await settler.locator('.recruit-current-cost [aria-label="1 Mood"]').count(), 1);
    assert.equal(await settler.locator('.recruit-standard-cost [aria-label="2 Food"]').count(), 1);
    assert.ok((await settler.boundingBox()).height < 195, 'Recruitment card stays compact');
    await page.screenshot({ path: `/tmp/clash-recruit-options-${width}.png` });
    state.players[seat].resources.food = 3;
    const resourcesBefore = structuredClone(state.players[seat].resources);
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(state), seat),
    );
    await page.locator('.city-content').evaluate((el) => (el.scrollTop = 24));
    const scrollBefore = await page.locator('.city-content').evaluate((el) => el.scrollTop);
    await page.getByRole('button', { name: 'Add Settler', exact: true }).click();
    assert.equal(await page.locator('.city-content').evaluate((el) => el.scrollTop), scrollBefore);
    for (const name of ['Settler', 'Infantry', 'Infantry'])
      await page.getByRole('button', { name: `Add ${name}`, exact: true }).click();
    const query = { kind: 'recruit', city, units: { settlers: 2, infantry: 2 }, replaced: [] };
    const quote = JSON.parse(engine.webQuery(JSON.stringify(state), seat, JSON.stringify(query)));
    assert.deepEqual(compact(quote.basePayment), { food: 6, ore: 2 });
    assert.deepEqual(compact(quote.payment), { food: 3, ore: 1, mood_tokens: 2 });
    assert.ok(quote.costOptions.includes('Sanitation') && quote.costOptions.includes('Draft'));
    const total = page.locator('.recruit-pay-total');
    for (const label of ['3 Food', '1 Ore', '2 Mood'])
      assert.equal(await total.locator(`[aria-label="${label}"]`).count(), 1);
    assert.ok(await page.locator('.city-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.locator('.recruit-confirm').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.locator('.city-content').evaluate((el) => (el.scrollTop = 0));
    await page.screenshot({ path: `/tmp/clash-recruit-costs-${width}.png` });
    await page.getByRole('button', { name: 'Recruit 4 units', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.city-dialog'));
    assert.equal(sent, 1);
    for (const [resource, spent] of Object.entries(compact(quote.payment)))
      assert.equal(state.players[seat].resources[resource] ?? 0, resourcesBefore[resource] - spent);
    await page.getByRole('button', { name: 'Research tree', exact: true }).click();
    await page.getByRole('combobox', { name: 'Filter advances', exact: true }).selectOption('all');
    const protection = page.locator('#research-Totalitarianism');
    await protection.scrollIntoViewIfNeeded();
    assert.match(await protection.innerText(), /your cities containing your army units/);
    await protection.locator('.research-pick').click();
    assert.match(
      await page.locator('.research-detail').innerText(),
      /your cities containing your army units/,
    );
    assert.ok(await page.locator('.research-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-research-ownership-${width}.png` });
    await page.getByRole('button', { name: 'Close research', exact: true }).click();
    state = structuredClone(initial);
    state.players[seat].advances = state.players[seat].advances.filter((a) => a !== 'Draft');
    state.players[seat].resources.gold = 1;
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(state), seat),
    );
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    const infantry = page.getByRole('article', { name: 'Infantry', exact: true });
    assert.match(await infantry.innerText(), /Gold can replace resources/);
    assert.equal(await infantry.locator('.recruit-standard-cost [aria-label="1 Food"]').count(), 1);
    assert.equal(await infantry.locator('.recruit-current-cost [aria-label="1 Gold"]').count(), 1);
    await page.getByRole('button', { name: 'Add Infantry', exact: true }).click();
    assert.equal(await page.locator('.recruit-pay-total [aria-label="1 Gold"]').count(), 1);
    await page.screenshot({ path: `/tmp/clash-recruit-gold-${width}.png` });
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: recruitment costs, gold substitution, single-use discounts, actual payment, compact layout, tab scroll, and research ownership verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
