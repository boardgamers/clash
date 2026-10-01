import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
    [320, 640],
    [844, 390],
  ]) {
    let state = await engine.init(3, [], { civilization: 'DraftThree' }, 'draft-mobile-preview', {});
    for (let i = 0; i < 3; i++)
      state = engine.setPlayerMetaData(state, i, { name: ['Ada', 'Lin', 'Sam'][i] });
    const pages = [],
      errors = [];
    async function emit(page, seat) {
      await page.evaluate(
        ({ state, seat }) => {
          host.emit('player', { index: seat });
          host.emit('state', state);
        },
        { state: engine.stripSecret(state, seat), seat },
      );
    }
    async function open(seat) {
      const page = await browser.newPage({ viewport: { width, height } });
      pages.push({ page, seat });
      page.on('pageerror', (e) => errors.push(e.message));
      await page.exposeFunction('applyMove', async (action) => {
        state = engine.tryMove(state, action, seat);
        await Promise.all(pages.map(({ page, seat }) => emit(page, seat)));
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
      await page.evaluate(() => {
        window.host = clash3d.launch('#app');
        host.on('move', (a) => window.applyMove(a));
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
      });
      await emit(page, seat);
      await page.getByRole('dialog', { name: 'Choose your civilization' }).waitFor();
      return page;
    }
    const a = await open(0),
      b = await open(1),
      spectator = await open(undefined);
    const picker = (page) => page.getByRole('dialog', { name: 'Choose your civilization' });
    const choices = (page) =>
      picker(page).getByRole('group', { name: 'Available civilizations' }).getByRole('button');
    assert.equal(await choices(a).count(), 3);
    assert.equal(await choices(b).count(), 3);
    const offeredA = await choices(a).allTextContents(),
      offeredB = await choices(b).allTextContents();
    assert.equal(new Set([...offeredA, ...offeredB]).size, 6);
    assert.equal(await spectator.locator('.faction-choices').count(), 0);
    await choices(a).nth(1).click();
    const chosen = (await choices(a).nth(1).innerText()).trim();
    // Another player can submit first without displacing this player's selection.
    await picker(b)
      .getByRole('button', { name: /Lock in/ })
      .click();
    await picker(b)
      .getByRole('heading', { name: /locked in/ })
      .waitFor();
    assert.equal(await choices(a).nth(1).getAttribute('aria-pressed'), 'true');
    await picker(a).getByRole('tab', { name: 'Leaders' }).click();
    assert.equal(await picker(a).getByRole('tabpanel', { name: 'Leaders' }).locator('article').count(), 3);
    await picker(a).getByRole('tab', { name: 'Advances' }).click();
    const button = picker(a).getByRole('button', { name: `Lock in ${chosen}` });
    await button.scrollIntoViewIfNeeded();
    const rect = await button.boundingBox();
    assert(rect && rect.width < width - 50 && rect.y + rect.height <= height + 1);
    assert.equal(await picker(a).evaluate((el) => el.scrollWidth <= el.clientWidth + 1), true);
    assert.equal(await picker(a).getByRole('button', { name: 'Minimize to see the map' }).isVisible(), false);
    await a.screenshot({ path: `/tmp/clash-civdraft-${width}.png` });
    await button.click();
    await picker(a)
      .getByRole('heading', { name: `${chosen} locked in` })
      .waitFor();
    await a.reload();
    await a.evaluate(() => {
      window.host = clash3d.launch('#app');
      host.emit('theme', { dark: true });
      host.emit('preferences', { sound: false });
    });
    await emit(a, 0);
    await picker(a)
      .getByRole('heading', { name: `${chosen} locked in` })
      .waitFor();
    assert.equal(await choices(b).count(), 0);
    assert.equal(JSON.parse(state).map.tiles.length, 0);
    const last = JSON.parse(engine.webView(engine.stripSecret(state, 2), 2)).civilizations[0];
    state = engine.tryMove(state, JSON.stringify(last.action), 2);
    await Promise.all(pages.map(({ page, seat }) => emit(page, seat)));
    await picker(a).waitFor({ state: 'detached' });
    await picker(b).waitFor({ state: 'detached' });
    assert.equal(await a.locator('.player-card').count(), 3);
    assert.equal(JSON.parse(state).state, 'Playing');
    assert.ok(JSON.parse(state).map.tiles.length > 0);
    assert.deepEqual(errors, []);
    for (const { page } of pages) await page.close();
    console.log(
      `${width}x${height}: independent private picks, preserved selection, waiting/reconnect, spectator privacy, compact lock button and simultaneous reveal verified.`,
    );
  }
} finally {
  await browser.close();
}
