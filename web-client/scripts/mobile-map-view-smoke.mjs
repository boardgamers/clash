import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'mapview-ui', {})).players.slice(2);
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
    [390, 740],
    [320, 640],
    [844, 390],
  ]) {
    let state = await fixture('advances/collect_free_economy'),
      seat = 0,
      sent = [];
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, seat));
      await emit();
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
    async function emit() {
      await page.evaluate(
        ({ state, seat }) => {
          host.emit('player', { index: seat });
          host.emit('state', state);
        },
        { state: engine.stripSecret(JSON.stringify(state), seat), seat },
      );
    }
    await page.goto('http://clash.test');
    await page.evaluate(() => {
      window.host = clash3d.launch('#app');
      host.on('move', (a) => window.applyMove(a));
      host.emit('theme', { dark: true });
      host.emit('preferences', { sound: false });
    });
    await emit();
    const minimize = (panel) => panel.getByRole('button', { name: 'Minimize to see the map', exact: true });
    const restore = page.locator('.restore-mobile-panel');
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    const collect = page.locator('.board-collection');
    await page.locator('.collect-map-label').first().click();
    const selected = await page.locator('.collect-map-label.selected').count();
    const total = await page.locator('.collect-submit > .resource-amount').innerText();
    await minimize(collect).click();
    assert(await collect.isHidden());
    assert(await restore.isVisible());
    const canvas = page.locator('canvas[aria-label*="civilization map"]');
    assert(await canvas.isVisible());
    assert.equal(await page.locator('dialog:modal').count(), 0);
    await page.screenshot({ path: `/tmp/clash-minimized-collect-${width}.png` });
    // Orbiting must not close or submit the hidden selection.
    const board = await canvas.boundingBox();
    await page.mouse.move(board.x + board.width * 0.45, board.y + board.height * 0.35);
    await page.mouse.down();
    await page.mouse.move(board.x + board.width * 0.6, board.y + board.height * 0.35, { steps: 4 });
    await page.mouse.up();
    await restore.click();
    assert.equal(await page.locator('.collect-map-label.selected').count(), selected);
    assert.equal(await page.locator('.collect-submit > .resource-amount').innerText(), total);
    assert.equal(sent.length, 0);
    await page.getByRole('button', { name: 'Close action', exact: true }).click();
    // A native modal must release the browser's modal/inert state without firing
    // its application close handler and destroying local form values.
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    const city = page.locator('dialog.city-dialog');
    await city.getByRole('button', { name: 'Recruit', exact: true }).click();
    await city.getByRole('button', { name: 'Add Settler', exact: true }).click();
    const units = await city.locator('.recruit-confirm').innerText();
    await city.locator('.city-content').evaluate((el) => (el.scrollTop = 100));
    const scroll = await city.locator('.city-content').evaluate((el) => el.scrollTop);
    await minimize(city).click();
    assert.equal(await page.locator('dialog:modal').count(), 0);
    assert.equal(await city.count(), 1);
    await canvas.click({ position: { x: board.width * 0.5, y: board.height * 0.4 } });
    assert.equal(await city.count(), 1, 'Board inspection must not cancel the hidden city form');
    await restore.click();
    assert(await city.evaluate((el) => el.matches(':modal')));
    assert.equal(await city.locator('.recruit-confirm').innerText(), units);
    assert.equal(await city.locator('.city-content').evaluate((el) => el.scrollTop), scroll);
    await page.screenshot({ path: `/tmp/clash-restored-city-${width}.png` });
    await city.getByRole('button', { name: 'Happiness', exact: true }).click();
    assert(
      await city.evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
      'Happiness header fits with minimize',
    );
    await city.getByRole('button', { name: 'Close city management', exact: true }).click();
    await page.getByRole('button', { name: 'Research tree', exact: true }).click();
    const research = page.locator('dialog.research-dialog');
    await research.getByRole('textbox', { name: 'Search research', exact: true }).fill('Fishing');
    await minimize(research).click();
    await page.getByRole('button', { name: 'How to play', exact: true }).click();
    const help = page
      .locator('dialog.field-guide')
      .filter({ has: page.getByRole('button', { name: 'Close field guide', exact: true }) });
    await help.waitFor();
    assert(await restore.isHidden(), 'A newly opened panel restores the UI');
    await help.getByRole('button', { name: 'Close field guide', exact: true }).click();
    assert(await research.evaluate((el) => el.matches(':modal')));
    assert.equal(
      await research.getByRole('textbox', { name: 'Search research', exact: true }).inputValue(),
      'Fishing',
    );
    await minimize(research).click();
    await page.keyboard.press('Escape');
    assert(await research.evaluate((el) => el.matches(':modal')));
    await minimize(research).click();
    await page.setViewportSize({ width: 1400, height: 900 });
    assert(await restore.isHidden());
    assert(await minimize(research).isHidden());
    assert.equal(
      await research.getByRole('textbox', { name: 'Search research', exact: true }).inputValue(),
      'Fishing',
    );
    await page.setViewportSize({ width, height });
    await research.getByRole('button', { name: 'Close research', exact: true }).click();
    state = await fixture('tactics_cards/peltasts');
    state.players[0].action_cards = [];
    state.players[1].action_cards = [4];
    seat = 1;
    state = JSON.parse(
      engine.tryMove(
        JSON.stringify(state),
        JSON.stringify({ Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }),
        0,
      ),
    );
    await emit();
    const tactics = page.getByRole('region', { name: 'Tactics', exact: true });
    await tactics.getByRole('button', { name: 'Select Peltasts', exact: true }).click();
    await minimize(tactics).click();
    await restore.click();
    assert.equal(
      await tactics
        .getByRole('button', { name: 'Select Peltasts', exact: true })
        .getAttribute('aria-pressed'),
      'true',
    );
    await minimize(tactics).click();

    // Start a real exploration with two legal orientations.
    state = await fixture('movement/explore_resolution');
    seat = engine.currentPlayer(JSON.stringify(state));
    let view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(state), seat), seat));
    state = JSON.parse(
      engine.tryMove(
        JSON.stringify(state),
        JSON.stringify(view.settlers[0].destinations.find((d) => d.position === 'D6').action),
        seat,
      ),
    );
    view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(state), seat), seat));
    const exploration = view.explorationDecision;
    await emit();
    const panel = page.getByRole('region', { name: 'Explore terrain placement', exact: true });
    await panel.waitFor();
    assert(await restore.isHidden(), 'A new engine decision restores the panel');
    assert.doesNotMatch(await panel.innerText(), /\b[A-Z]+\d+\b/);
    assert.deepEqual(
      (await page.locator('.map-exploration-label').getAttribute('data-positions')).split(' ').sort(),
      exploration.choices[0].tiles.map(([p]) => p).sort(),
    );
    await panel.getByRole('button', { name: 'Terrain placement 2', exact: true }).click();
    await panel.getByRole('button', { name: 'Locate the region being explored', exact: true }).click();
    await page.waitForTimeout(100); // Let the map's resize frame catch up with the tray layout.
    const panelBounds = await panel.boundingBox();
    const confirmBounds = await panel.getByRole('button', {name:'Confirm',exact:true}).boundingBox();
    assert(confirmBounds.y + confirmBounds.height <= panelBounds.y + panelBounds.height, 'Exploration confirmation is visible without scrolling');
    const labelBounds = await page.locator('.map-exploration-label').boundingBox();
    assert(labelBounds.x >= 0 && labelBounds.x + labelBounds.width <= width, 'Explored region stays on screen');
    assert(labelBounds.y + labelBounds.height <= panelBounds.y || labelBounds.x >= panelBounds.x + panelBounds.width, 'Region label remains clear of the orientation panel');
    await page.screenshot({ path: `/tmp/clash-exploring-region-${width}.png` });
    await minimize(panel).click();
    assert(await page.locator('.map-exploration-label').isVisible());
    await page.screenshot({ path: `/tmp/clash-exploring-minimized-${width}.png` });
    await restore.click();
    assert.equal(
      await panel
        .getByRole('button', { name: 'Terrain placement 2', exact: true })
        .getAttribute('aria-pressed'),
      'true',
    );
    assert.equal(sent.length, 0);
    await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
    await panel.waitFor({ state: 'detached' });
    assert.deepEqual(sent, [exploration.choices[1].action]);
    for (const [position, terrain] of exploration.choices[1].tiles)
      assert.deepEqual(state.map.tiles.find(([p]) => p === position)?.[1], terrain);
    assert(await page.locator('.map-exploration-label').isHidden());
    assert(await restore.isHidden());
    assert.deepEqual(errors, []);
    console.log(
      `${width}x${height}: minimize/restore preserves collection, recruitment, search and exploration; modal stack, map gestures and revealed region verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
