import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const game = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/incidents/barbarians_move.outcome1.json', import.meta.url),
    'utf8',
  ),
);
const npcs = JSON.parse(await engine.init(2, [], {}, 'barbarians-smoke', {})).players.slice(2);
for (const npc of npcs)
  if (!game.players.some((p) => p.civilization === npc.civilization))
    game.players.push({ ...npc, id: game.players.length });
// Inspect completed public outcomes without a pending fixture decision intercepting map taps.
game.state = 'Playing';
game.events = [];
game.current_player_index = 0;
const state = engine.stripSecret(JSON.stringify(game), 0);
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
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    // Serve the built viewer entirely in memory; no local server or network access required.
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
        host.emit('player', { index: 0 });
        host.emit('theme', { dark });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state, dark },
    );
    await page.getByRole('button', { name: /Select Barbarians city B3/ }).click();
    const details = page.getByRole('region', { name: 'Barbarian effects' });
    await details.waitFor();
    assert.match(await details.innerText(), /Army 1\/4/);
    assert.match(await details.innerText(), /gain 1 infantry, up to 4 units/);
    assert.ok(await details.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-barbarians-tile-${width}.png` });
    await page.getByRole('button', { name: 'Close tile actions' }).click();
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    const reason = page.getByText(/Reinforced after barbarian movement/);
    await reason.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `/tmp/clash-barbarians-journal-${width}.png` });
    assert.ok(await page.locator('.journal').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    await page.getByRole('button', { name: 'How to play', exact: true }).click();
    await page.getByRole('heading', { name: 'Victory and game end' }).waitFor();
    const guide = page.locator('.field-guide');
    assert.match(await guide.innerText(), /highest total victory score wins/);
    assert.match(await guide.innerText(), /any player has no cities/);
    assert.ok(await guide.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-victory-guide-${width}.png` });
    await page.getByRole('button', { name: 'Close field guide' }).click();
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    const cavalry = page.locator('.recruit-row').filter({ has: page.getByText('Cavalry', { exact: true }) });
    const elephant = page
      .locator('.recruit-row')
      .filter({ has: page.getByText('Elephant', { exact: true }) });
    await elephant.scrollIntoViewIfNeeded();
    assert.match(await cavalry.innerText(), /\+2 combat value on cavalry face/);
    assert.match(await elephant.innerText(), /Blocks 1 hit on elephant face · 0 die value/);
    assert.ok(await page.locator('.city-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: `/tmp/clash-recruit-roles-${width}.png` });
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: barbarian details, reinforcement cause, victory guide and recruit roles verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
