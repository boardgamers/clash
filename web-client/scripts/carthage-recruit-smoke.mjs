// Synthetic/offline real-engine and production-viewer combined recruitment check.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'carthage-combined-recruit', {});
for (const civilization of ['Carthage', 'Rome'])
  raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: civilization }), engine.currentPlayer(raw));
const initial = JSON.parse(raw), seat = engine.currentPlayer(raw), p = initial.players[seat];
p.advances.push('Husbandry', 'Draft', 'Fishing', 'Tactics');
p.cities = [{ position: 'D2', mood_state: 'Happy', city_pieces: { port: seat }, port_position: 'D3' }];
p.units = [];
for (const tile of initial.map.tiles) tile[1] = tile[0] === 'D2' ? 'Fertile' : 'Water';
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const width of [1400, 390]) {
    for (const payment of [
      { food: 1, gold: 2 },
      { food: 1, gold: 1, mood_tokens: 1 },
      { food: 1, gold: 1, culture_tokens: 1 },
    ]) {
      const state = structuredClone(initial);
      // Keep the regular payment available too, so the picker must be used.
      state.players[seat].resources = { wood: 2, ...payment };
      const stripped = engine.stripSecret(JSON.stringify(state), seat);
      const offer = JSON.parse(engine.webQuery(stripped, seat, JSON.stringify({
        kind: 'recruit', city: 'D2', units: { elephants: 1, ships: 1 }, replaced: [], payment,
      })));
      assert.deepEqual(offer.payment, payment);
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname !== 'clash.test') return route.abort();
        if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body:
          '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
        if (!['/viewer.js', '/server_bg.wasm'].includes(url.pathname)) return route.abort();
        return route.fulfill({ contentType: url.pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
          body: await readFile(new URL('../dist' + url.pathname, import.meta.url)) });
      });
      await page.goto('http://clash.test/');
      await page.evaluate(({ state, seat }) => {
        window.host = clash3d.launch('#app'); window.sent = [];
        host.on('move', (action) => sent.push(JSON.parse(action)));
        host.emit('player', { index: seat });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      }, { state: stripped, seat });
      await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
      const city = page.getByRole('dialog');
      await city.getByRole('button', { name: 'Recruit', exact: true }).click();
      await city.getByRole('button', { name: 'Add Elephant', exact: true }).click();
      await city.getByRole('button', { name: 'Add Ship', exact: true }).click();
      const picker = city.getByRole('group', { name: 'Recruitment payment', exact: true });
      await picker.waitFor();
      if (await picker.locator('select').count()) {
        await picker.getByLabel('Wood amount for Recruitment payment', { exact: true }).selectOption('0');
        if (payment.culture_tokens)
          await picker.getByLabel('Culture amount for Recruitment payment', { exact: true }).selectOption('1');
        if (payment.mood_tokens)
          await picker.getByLabel('Mood amount for Recruitment payment', { exact: true }).selectOption('1');
        await picker.getByLabel('Food amount for Recruitment payment', { exact: true }).selectOption('1');
        await picker.getByLabel('Gold amount for Recruitment payment', { exact: true }).selectOption(String(payment.gold));
      } else {
        const name = 'Pay ' + Object.entries(offer.payment).map(([key, value]) =>
          `${value} ${key === 'mood_tokens' ? 'mood' : key === 'culture_tokens' ? 'culture' : key}`).join(' + ');
        await picker.getByRole('button', { name, exact: true }).click();
      }
      await page.evaluate((state) => host.emit('state', state), stripped);
      assert.deepEqual(await page.evaluate(() => sent), []);
      await city.getByRole('button', { name: 'Recruit 2 units', exact: true }).click();
      await page.waitForFunction(() => sent.length === 1);
      const action = await page.evaluate(() => sent[0]);
      assert.deepEqual(action.Playing.Recruit.payment, payment);
      const after = JSON.parse(engine.tryMove(JSON.stringify(state), JSON.stringify(action), seat));
      assert.equal(after.players[seat].units.filter((u) => u.unit_type === 'Elephant').length, 1);
      assert.equal(after.players[seat].units.filter((u) => u.unit_type === 'Ship').length, 1);
      for (const [key, amount] of Object.entries(payment))
        assert.equal(after.players[seat].resources[key] ?? 0, state.players[seat].resources[key] - amount);
      assert.deepEqual(errors, []);
      console.log(`${width}px: combined Elephant + Ship ${JSON.stringify(payment)} selected, retained and executed`);
      await page.close();
    }
  }
} finally { await browser.close(); }
