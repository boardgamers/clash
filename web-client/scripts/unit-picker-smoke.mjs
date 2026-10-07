import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'unit-picker', {})).players.slice(2);
const base = JSON.parse(
  await readFile(
    new URL('../../server/tests/test_games/incidents/pandemics/black_death.outcome.json', import.meta.url),
    'utf8',
  ),
);
for (const npc of npcs)
  if (!base.players.some((p) => p.civilization === npc.civilization))
    base.players.push({ ...npc, id: base.players.length });
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
async function tapHex(page, position) {
  const target = page.locator(`.map-hit-target[data-position="${position}"]`);
  await target.waitFor({ state: 'attached' });
  await page.waitForTimeout(150);
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  const box = await target.boundingBox();
  assert.ok(box, `Hex ${position} is on the board`);
  assert.equal(await target.evaluate((el) => getComputedStyle(el).opacity), '0');
  assert.equal(await target.evaluate((el) => getComputedStyle(el).pointerEvents), 'none');
  assert.equal(await target.textContent(), '');
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
}
const intersects = (a, b) =>
  a &&
  b &&
  Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x) > 1 &&
  Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y) > 1;
try {
  for (const [width, height] of [
    [1400, 900],
    [910, 721],
    [390, 844],
    [390, 650],
    [320, 570],
    [320, 700],
    [900, 450],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [],
      sent = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('captureMove', (move) => sent.push(JSON.parse(move)));
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
        host.on('move', (m) => window.captureMove(m));
        host.emit('player', { index: 0 });
        host.emit('theme', { dark });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: engine.stripSecret(JSON.stringify(base), 0), dark: width < 500 },
    );
    const decision = page.locator('.unit-decision');
    await decision.waitFor();
    assert.equal(await page.locator('.decision-piece-label').count(), 0);
    await page.locator('.unit-portrait img').first().waitFor();
    assert.equal(await page.locator('.unit-choice').count(), 7);
    await page.waitForTimeout(400);
    await page.screenshot({ path: `/tmp/clash-unit-picker-${width}x${height}.png` });
    assert.ok((await decision.boundingBox()).height < 240, `${width}: casualty selection stays compact`);
    assert.doesNotMatch(await decision.innerText(), /#[0-9]|\b[A-F][0-9]\b/);
    for (const selector of [
      '.map-controls',
      '.table-tools',
      '.board-toolbar',
      '.city-dock',
      '.player-list',
    ]) {
      const el = page.locator(selector);
      if (await el.isVisible())
        assert.ok(
          !intersects(await decision.boundingBox(), await el.boundingBox()),
          `${width}: decision overlaps ${selector}`,
        );
    }
    await decision.locator('.event-rules summary').click();
    const rulesConfirm = await decision.getByRole('button', { name: 'Confirm', exact: true }).boundingBox();
    assert.ok(
      rulesConfirm && rulesConfirm.y + rulesConfirm.height <= height,
      `${width}: rules do not obscure Confirm`,
    );
    await decision.locator('.event-rules summary').click();
    const confirm = decision.getByRole('button', { name: 'Confirm', exact: true });
    const box = await confirm.boundingBox();
    assert.ok(box && box.y >= 0 && box.y + box.height <= height, `${width}: Confirm remains on screen`);
    assert.equal(await confirm.isDisabled(), true);
    await decision.getByRole('button', { name: 'Infantry', exact: true }).nth(0).click();
    assert.equal(await decision.locator('.unit-choice[aria-pressed=true]').count(), 1);
    await decision.getByRole('button', { name: 'Infantry', exact: true }).nth(1).click();
    assert.equal(await decision.locator('.unit-choice[aria-pressed=true]').count(), 1);
    if (width <= 760) {
      await page.locator('.table-tools').getByRole('button', { name: 'Open journal', exact: true }).click();
      assert.equal(await decision.isVisible(), false);
      await page.getByRole('button', { name: 'Close table activity', exact: true }).click();
      assert.equal(await decision.isVisible(), true);
      assert.equal(await decision.locator('.unit-choice[aria-pressed=true]').count(), 1);
    }
    await decision.getByRole('button', { name: 'Confirm · 1/1', exact: true }).click();
    assert.equal(sent.length, 1);
    const after = JSON.parse(engine.tryMove(JSON.stringify(base), JSON.stringify(sent[0]), 0));
    assert.equal(
      after.players[0].units.some((u) => u.id === 1),
      false,
    );
    assert.equal(
      after.players[0].units.some((u) => u.id === 0),
      true,
    );

    const multiple = structuredClone(base);
    multiple.events[0].handler.request.SelectUnits.needed = { start: 2, end: 2 };
    multiple.players[0].units.find((u) => u.id === 7).position = 'B3';
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(multiple), 0),
    );
    await decision.getByRole('button', { name: 'Infantry', exact: true }).nth(0).click();
    await tapHex(page, 'B3');
    assert.equal(await decision.locator('.unit-choice').count(), 1);
    // B3 has a single eligible unit: tapping it on the map chooses it (confirmation still required).
    assert.match(await decision.innerText(), /2 \/ 2 selected/);
    assert.equal(
      await decision.getByRole('button', { name: 'Settler', exact: true }).getAttribute('aria-pressed'),
      'true',
    );
    assert.equal(sent.length, 1);
    await decision.getByRole('button', { name: 'Show next group of units', exact: true }).click();
    assert.equal(
      await decision
        .getByRole('button', { name: 'Infantry', exact: true })
        .nth(0)
        .getAttribute('aria-pressed'),
      'true',
    );
    assert.equal(
      await decision.getByRole('button', { name: 'Infantry', exact: true }).nth(1).isDisabled(),
      true,
    );
    await decision.getByRole('button', { name: 'Confirm · 2/2', exact: true }).click();
    const multiAfter = JSON.parse(engine.tryMove(JSON.stringify(multiple), JSON.stringify(sent.at(-1)), 0));
    assert.equal(
      multiAfter.players[0].units.some((u) => [0, 7].includes(u.id)),
      false,
    );

    const movement = JSON.parse(
      await readFile(
        new URL('../../server/tests/test_games/movement/ship_transport.outcome.json', import.meta.url),
        'utf8',
      ),
    );
    for (const npc of npcs)
      if (!movement.players.some((p) => p.civilization === npc.civilization))
        movement.players.push({ ...npc, id: movement.players.length });
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(movement), 0),
    );
    const movePanel = page.getByRole('region', { name: 'Unit movement', exact: true });
    await movePanel.waitFor();
    await page.waitForTimeout(400);
    await tapHex(page, 'D2');
    const ship = movePanel.getByRole('button', { name: 'Ship', exact: true });
    await ship.click();
    await ship.click();
    assert.equal(
      await movePanel.locator('.unit-choice').count(),
      3,
      'Deselecting every unit keeps the current hex visible',
    );
    await ship.click();
    const passenger = movePanel.getByRole('button', { name: /Cavalry.*Disembark/ });
    await passenger.click();
    assert.equal(await ship.getAttribute('aria-pressed'), 'false');
    assert.equal(await passenger.getAttribute('aria-pressed'), 'true');
    assert.match(await movePanel.locator('h2').innerText(), /Disembark/);
    await page.waitForTimeout(300);
    await page.screenshot({ path: `/tmp/clash-unit-movement-${width}x${height}.png` });
    assert.ok((await movePanel.boundingBox()).height < 195, `${width}: movement stays compact`);
    assert.doesNotMatch(
      await movePanel.innerText(),
      /#[0-9]|\b[A-F][0-9]\b/,
      'Selection needs no IDs or coordinates',
    );
    for (const selector of [
      '.map-controls',
      '.table-tools',
      '.board-toolbar',
      '.city-dock',
      '.player-list',
    ]) {
      const el = page.locator(selector);
      if (await el.isVisible())
        assert.ok(
          !intersects(await movePanel.boundingBox(), await el.boundingBox()),
          `${width}: movement overlaps ${selector}`,
        );
    }
    await tapHex(page, 'C2');
    await movePanel.getByRole('button', { name: 'Disembark', exact: true }).click();
    const disembarked = JSON.parse(engine.tryMove(JSON.stringify(movement), JSON.stringify(sent.at(-1)), 0));
    assert.ok(
      disembarked.players[0].units.some((u) => u.id === 1 && u.position !== 'D2'),
      'Selected passenger reaches land',
    );
    assert.ok(
      disembarked.players[0].units.find((u) => u.id === 7).carried_units.some((u) => u.id === 2),
      'Other passenger stays aboard',
    );
    // Tile-only decisions also stay on the board and preserve keyboard selection.
    const tiles = JSON.parse(
      await readFile(
        new URL('../../server/tests/test_games/incidents/exhausted_land.outcome.json', import.meta.url),
        'utf8',
      ),
    );
    for (const npc of npcs)
      if (!tiles.players.some((p) => p.civilization === npc.civilization))
        tiles.players.push({ ...npc, id: tiles.players.length });
    await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(tiles), 0));
    const tilePanel = page.locator('.board-decision');
    await tilePanel.waitFor();
    await page.waitForTimeout(300);
    assert.equal(await tilePanel.locator('.decision-options').count(), 0);
    assert.doesNotMatch(await tilePanel.innerText(), /\b[A-F][0-9]\b/);
    await tapHex(page, 'B1');
    assert.match(await tilePanel.locator('.map-selection-count').innerText(), /1\/1/);
    const keyboardHex = page.locator('.map-hit-target[data-position="B2"]');
    await keyboardHex.focus();
    await keyboardHex.press('Enter');
    assert.equal(await keyboardHex.getAttribute('aria-pressed'), 'true');
    await tilePanel.getByRole('button', { name: 'Confirm · 1/1', exact: true }).click();
    const exhausted = JSON.parse(engine.tryMove(JSON.stringify(tiles), JSON.stringify(sent.at(-1)), 0));
    assert.equal(typeof exhausted.map.tiles.find(([p]) => p === 'B2')[1], 'object');
    assert.deepEqual(errors, []);
    console.log(
      `Unit picker ${width}×${height}: casualties across hexes, limits, passenger movement and unobstructed controls`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
