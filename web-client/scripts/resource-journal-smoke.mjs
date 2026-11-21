// Offline real built-viewer regression. Uses only checked-in engine fixtures;
// no production requests, private games or mutations of resource balances.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const raw = JSON.parse(await readFile(new URL('../../server/tests/test_games/incidents/pandemics/pandemics.outcome3.json', import.meta.url), 'utf8'));
const balances = raw.players.map(p => p.resources);
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const [width, height] of [[1400, 900], [390, 740]]) {
    for (const repeated of [false, true]) {
      const state = structuredClone(raw);
      const actions = state.log[0].rounds[0].turns[0].actions;
      const payment = actions.at(-1);
      assert.equal(payment.items[0].Resources.balance, 'Pay');
      assert.equal(payment.items[1].Text, 'Lose 1 culture token');
      // Minimal redacted reproduction of the reported wood/ore payment echoes.
      // The surrounding board is a public repository fixture, not the private game.
      payment.items = ['wood', 'ore'].flatMap((resource, player) => [
        { player, origin: { Incident: 49 }, Resources: { balance: 'Pay', resources: { [resource]: 1 } } },
        { player, origin: { Incident: 49 }, Text: `Lose 1 ${resource}` },
      ]);
      if (repeated) payment.items.push(...structuredClone(payment.items));
      const publicState = engine.stripSecret(JSON.stringify(state), 0);
      assert.deepEqual(JSON.parse(publicState).players.map(p => p.resources), balances);
      const page = await browser.newPage({ viewport: { width, height } });
      const errors = [];
      page.on('pageerror', e => errors.push(e.message));
      await page.route('**/*', async route => {
        const path = new URL(route.request().url()).pathname;
        if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
        if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
        return route.fulfill({ contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body: await readFile(new URL('../dist' + path, import.meta.url)) });
      });
      await page.goto('http://clash.test');
      await page.evaluate(state => {
        window.host = clash3d.launch('#app');
        host.emit('player', { index: 0 });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      }, publicState);
      await page.getByRole('button', { name: 'Open journal', exact: true }).click();
      const event = page.locator('.journal-event').filter({ hasText: /Pandemic/ });
      await event.waitFor();
      const badges = event.locator('.journal-delta[aria-label*="wood"], .journal-delta[aria-label*="ore"]');
      for (const resource of ['wood', 'ore']) assert.equal(await event.locator(`.journal-delta[aria-label*="${resource}"]`).count(), repeated ? 2 : 1);
      for (const badge of await badges.all()) assert.match(await badge.innerText(), /−1/);
      await event.locator('.event-rules summary').click();
      assert.match(await event.locator('.event-rules').innerText(), /Every player loses/);
      assert.deepEqual(JSON.parse(publicState).players.map(p => p.resources), balances);
      await page.screenshot({ path: `/tmp/resource-journal-${width}${repeated ? '-repeated' : ''}.png` });
      assert.deepEqual(errors, []);
      console.log(`${width}px ${repeated ? 'two real payments' : 'one payment plus prose'}: ${await badges.count()} wood/ore badge(s), balances unchanged`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
