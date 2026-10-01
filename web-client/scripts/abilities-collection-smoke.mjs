import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'abilities-ui', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
    [320, 640],
  ]) {
    let state = await fixture('advances/increase_happiness_sports'),
      sent = [];
    const before = structuredClone(state);
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, 0));
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(state), 0));
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
      (state) => {
        window.host = clash3d.launch('#app');
        host.on('move', (a) => window.applyMove(a));
        host.emit('player', { index: 0 });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      engine.stripSecret(JSON.stringify(state), 0),
    );
    await page.getByRole('button', { name: 'Abilities and cultural influence', exact: true }).click();
    const panel = page.getByRole('region', { name: 'Abilities and influence', exact: true });
    const sports = panel
      .locator('.ability-offer')
      .filter({ has: page.getByRole('heading', { name: 'Sports', exact: true }) });
    assert.equal(await sports.count(), 1);
    assert.doesNotMatch(await sports.innerText(), /\b[A-F][0-9]\b/);
    await sports.getByRole('button', { name: /Choose city/ }).click();
    await panel.getByText('Choose a highlighted city.', { exact: true }).waitFor();
    assert.equal(await page.locator('.map-hit-target').count(), 3);
    assert(await panel.getByRole('button', { name: /Use Sports/ }).isDisabled());
    assert.equal(sent.length, 0);
    // Keyboard and pointer both select existing board cities. No coordinate badges.
    const neutral = page.locator('.map-hit-target[data-position="B1"]');
    await neutral.focus();
    await neutral.press('Enter');
    assert.equal(await neutral.getAttribute('aria-pressed'), 'true');
    const angry = page.locator('.map-hit-target[data-position="B3"]');
    await page.waitForTimeout(200);
    const box = await angry.boundingBox();
    assert(box);
    assert.equal(await angry.evaluate((el) => getComputedStyle(el).opacity), '0');
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    assert.equal(await angry.getAttribute('aria-pressed'), 'true');
    assert.equal(await neutral.getAttribute('aria-pressed'), 'false');
    assert.equal(sent.length, 0);
    assert.doesNotMatch(await panel.innerText(), /\b[A-F][0-9]\b/);
    assert((await panel.boundingBox()).height < 245, 'City selection is a compact tray');
    assert(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-sports-map-${width}.png` });
    await panel.getByRole('button', { name: /Use Sports/ }).click();
    await page.getByRole('button', { name: 'Pay 2 culture', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.decision-panel'));
    assert.equal(sent.length, 2);
    assert.deepEqual(sent[0], { Playing: { Custom: { action: 'Sports', city: 'B3' } } });
    assert.equal(state.players[0].cities.find((c) => c.position === 'B3').mood_state, 'Happy');
    assert.equal(state.players[0].cities.find((c) => c.position === 'B1').mood_state, 'Neutral');
    assert.equal(state.players[0].resources.culture_tokens, before.players[0].resources.culture_tokens - 2);
    assert.equal(state.actions_left, before.actions_left - 1);
    state = await fixture('advances/collect_free_economy');
    sent = [];
    const collectBefore = structuredClone(state);
    await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(state), 0));
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    // Use an actual map yield badge, then switch action and back without reselecting it.
    const tile = page.locator('.collect-map-label').first();
    await tile.click();
    const selected = await page.locator('.collect-map-label.selected').count();
    assert(selected > 0);
    const total = await page.locator('.collect-submit > .resource-amount').innerText();
    await page
      .locator('.variant-picker')
      .getByRole('button', { name: /Free Economy/ })
      .click();
    assert.equal(await page.locator('.collect-map-label.selected').count(), selected);
    assert.equal(await page.locator('.collect-submit > .resource-amount').innerText(), total);
    await page
      .locator('.variant-picker')
      .getByRole('button', { name: /^Collect/ })
      .click();
    assert.equal(await page.locator('.collect-map-label.selected').count(), selected);
    await page
      .locator('.variant-picker')
      .getByRole('button', { name: /Free Economy/ })
      .click();
    assert.equal(sent.length, 0);
    await page.screenshot({ path: `/tmp/clash-free-economy-selection-${width}.png` });
    await page.locator('.collect-submit').click();
    await page.waitForFunction(
      () => !document.querySelector('.board-collection') && !document.querySelector('.decision-panel'),
    );
    assert.equal(state.players[0].resources.mood_tokens, collectBefore.players[0].resources.mood_tokens - 1);
    assert.equal(state.actions_left, collectBefore.actions_left);
    assert.equal(sent.length, 2); // Collection plus the existing automatic one-mood payment.
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: one Sports offer, city selection on the board, exact city/cost, and preserved Free Economy collection verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
