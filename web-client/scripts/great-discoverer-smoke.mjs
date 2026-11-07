import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { greatDiscovererFixture } from './great-discoverer-fixture.mjs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const width of [1400, 390]) {
    for (const scenario of ['food', 'gold', 'decline', 'poor', 'supply', 'no-irrigation']) {
      const f = greatDiscovererFixture(engine, {
        resources: scenario === 'poor' ? { gold: 1 } : scenario === 'gold' ? { gold: 2 } : { food: 2 },
        noCities: scenario === 'supply',
        irrigation: scenario !== 'no-irrigation',
        mixedTerrain: true,
      });
      const beforeExplore = f.reveal();
      const before = JSON.parse(f.state).players[1];
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [], sent = [];
      page.on('pageerror', e => errors.push(e.message));
      await page.exposeFunction('applyMove', async action => {
        const parsed = typeof action === 'string' ? JSON.parse(action) : action;
        f.apply(parsed);
        sent.push(parsed);
        await page.evaluate(state => host.emit('state', state), engine.stripSecret(f.state, 1));
      });
      await page.route('**/*', async route => {
        const path = new URL(route.request().url()).pathname;
        if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
        if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
        return route.fulfill({ contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body: await readFile(new URL('../dist' + path, import.meta.url)) });
      });
      await page.goto('http://clash.test/');
      await page.evaluate(state => {
        window.host = clash3d.launch('#app');
        host.on('move', action => window.applyMove(action));
        host.emit('player', { index: 1 });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      }, engine.stripSecret(f.state, 1));
      await page.getByRole('button', { name: 'Open journal', exact: true }).waitFor();
      const panel = page.locator('.decision-panel');
      const blocked = ['poor', 'supply'].includes(scenario);
      if (blocked) {
        assert.equal(await panel.count(), 0);
        assert.equal(await page.locator('.map-hit-target').count(), 0, 'no actionable placement dead end');
        await page.getByRole('button', { name: 'Open journal', exact: true }).click();
        const reason = scenario === 'poor' ? 'Player cannot afford to build a city' : 'You have reached your city limit';
        await page.getByText(reason, { exact: false }).first().waitFor();
        assert.deepEqual(before.resources, beforeExplore.players[1].resources);
        assert.ok(before.advances.includes('Fishing'));
        assert.deepEqual(sent, []);
      } else {
        await panel.waitFor();
        assert.match(await panel.innerText(), /Place a city for\s+2/);
        const targets = await page.locator('.map-hit-target').evaluateAll(els => els.map(e => e.dataset.position).sort());
        assert.deepEqual(targets, scenario === 'no-irrigation' ? ['B6'] : ['B6', 'B7']);
        assert.equal(await panel.getByRole('button', { name: 'Skip', exact: true }).isVisible(), true);
        if (['decline', 'no-irrigation'].includes(scenario)) {
          await panel.getByRole('button', { name: 'Skip', exact: true }).click();
          await panel.waitFor({ state: 'detached' });
          assert.deepEqual(sent, [{ Response: { SelectPositions: [] } }]);
          assert.deepEqual(JSON.parse(f.state).players[1].resources, before.resources);
          assert.deepEqual(JSON.parse(f.state).players[1].cities, before.cities);
        } else {
          const target = page.locator('.map-hit-target[data-position="B7"]');
          await target.waitFor({ state: 'attached' });
          await page.waitForTimeout(250);
          const box = await target.boundingBox();
          assert.ok(box, 'Barren is highlighted on the map');
          await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
          await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
          await panel.getByText('Pay to build the city', { exact: true }).waitFor();
          assert.equal(JSON.parse(f.state).players[1].cities.length, before.cities.length, 'location selection does not found for free');
          assert.equal(sent.length, 1);
          assert.equal(await panel.getByRole('button', { name: 'Decline', exact: true }).count(), 0);
          await page.screenshot({ path: `/tmp/clash-great-discoverer-${scenario}-${width}.png` });
          await panel.locator('.compact-payments button').click();
          await panel.waitFor({ state: 'detached' });
          const after = JSON.parse(f.state).players[1];
          assert.deepEqual(sent[1], { Response: { Payment: [scenario === 'gold' ? { gold: 2 } : { food: 2 }] } });
          assert.equal(after.cities.at(-1).position, 'B7');
          assert.equal(after.cities.length, before.cities.length + 1);
          assert.deepEqual(after.units, before.units);
        }
      }
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert.deepEqual(errors, []);
      console.log(`${width}px ${scenario}: engine-backed placement/payment/skip and eligibility passed`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
