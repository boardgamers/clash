// Offline real-engine + built-viewer regression. Run after build:bridge and build.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const initial = JSON.parse(await engine.init(2, [], { civilization: 'Random' }, 'payment-defaults', {}));
const seat = engine.currentPlayer(JSON.stringify(initial));
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const width of [1400, 390]) {
    for (const [resources, expected] of [
      [{ food: 2, gold: 2 }, { food: 2 }],
      [{ food: 1, gold: 2 }, { food: 1, gold: 1 }],
      [{ ideas: 1, food: 2, gold: 2 }, { ideas: 1, food: 1 }],
    ]) {
      const state = structuredClone(initial);
      state.players[seat].resources = resources;
      const stripped = engine.stripSecret(JSON.stringify(state), seat);
      const research = JSON.parse(engine.webView(stripped, seat)).advances.find((a) => a.id === 'Storage');
      assert.deepEqual(research.payment, expected);
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
      await page.getByRole('button', { name: 'Research tree', exact: true }).click();
      await page.locator('#research-Storage .research-pick').click();
      const picker = page.getByRole('group', { name: 'Research payment', exact: true });
      await picker.waitFor();
      if (research.payments.length <= 4) {
        const selected = picker.locator('[aria-pressed="true"]');
        assert.equal(await selected.count(), 1);
        for (const [resource, amount] of Object.entries(expected))
          assert.equal(await selected.locator(`[title="${amount} ${resource[0].toUpperCase() + resource.slice(1)}"]`).count(), 1);
        await picker.getByRole('button', { name: 'Pay 2 gold', exact: true }).click();
      } else {
        assert.equal(await picker.getByLabel('Food amount for Research payment').inputValue(), '1');
        assert.equal(await picker.getByLabel('Ideas amount for Research payment').inputValue(), '1');
        assert.equal(await picker.getByLabel('Gold amount for Research payment').inputValue(), '0');
        await picker.getByLabel('Gold amount for Research payment').selectOption('2');
      }
      // A normal host refresh must not reset a valid manual allocation.
      await page.evaluate((state) => host.emit('state', state), stripped);
      if (research.payments.length <= 4)
        assert.equal(await picker.getByRole('button', { name: 'Pay 2 gold', exact: true }).getAttribute('aria-pressed'), 'true');
      else assert.equal(await picker.getByLabel('Gold amount for Research payment').inputValue(), '2');
      assert.deepEqual(await page.evaluate(() => sent), []);
      await page.locator('.research-payment > .primary').click();
      await page.waitForFunction(() => sent.length === 1);
      const action = await page.evaluate(() => sent[0]);
      assert.deepEqual(action.Playing.Advance.payment, { gold: 2 });
      const after = JSON.parse(engine.tryMove(JSON.stringify(state), JSON.stringify(action), seat));
      assert.equal(after.players[seat].resources.gold ?? 0, 0);
      assert.equal(after.players[seat].resources.food ?? 0, resources.food);
      assert.deepEqual(errors, []);
      console.log(`${width}px ${JSON.stringify(resources)}: native default and persistent manual gold verified`);
      await page.close();
    }
  }
} finally { await browser.close(); }
