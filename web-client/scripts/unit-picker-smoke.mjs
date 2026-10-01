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
    await page.screenshot({ path: `/tmp/clash-unit-picker-${width}.png` });
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
    await decision.getByRole('button', { name: 'Infantry #1 at C2', exact: true }).click();
    assert.equal(await decision.locator('.unit-choice[aria-pressed=true]').count(), 1);
    await decision.getByRole('button', { name: 'Infantry #2 at C2', exact: true }).click();
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
    await decision.getByRole('button', { name: 'Infantry #1 at C2', exact: true }).click();
    await page.locator('.world-labels').getByRole('button', { name: 'Units at B3', exact: true }).click();
    assert.equal(await decision.locator('.unit-choice').count(), 1);
    assert.match(await decision.innerText(), /1 \/ 2 selected/);
    await decision.getByRole('button', { name: 'Settler #8 at B3', exact: true }).click();
    await decision
      .getByRole('group', { name: 'Unit locations', exact: true })
      .getByRole('button', { name: 'Units at C2', exact: true })
      .click();
    assert.equal(
      await decision
        .getByRole('button', { name: 'Infantry #1 at C2', exact: true })
        .getAttribute('aria-pressed'),
      'true',
    );
    assert.equal(
      await decision.getByRole('button', { name: 'Infantry #2 at C2', exact: true }).isDisabled(),
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
    await movePanel
      .getByRole('group', { name: 'Unit locations', exact: true })
      .getByRole('button', { name: 'Units at D2', exact: true })
      .click();
    const ship = movePanel.getByRole('button', { name: 'Ship #8 at D2', exact: true });
    await ship.click();
    await ship.click();
    assert.equal(
      await movePanel.locator('.unit-choice').count(),
      3,
      'Deselecting every unit keeps the current hex visible',
    );
    await ship.click();
    const passenger = movePanel.getByRole('button', { name: /Cavalry #2 at D2.*Disembark/ });
    await passenger.click();
    assert.equal(await ship.getAttribute('aria-pressed'), 'false');
    assert.equal(await passenger.getAttribute('aria-pressed'), 'true');
    assert.match(await movePanel.locator('h2').innerText(), /Disembark/);
    await page.waitForTimeout(300);
    await page.screenshot({ path: `/tmp/clash-unit-movement-${width}.png` });
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
    await movePanel.locator('.movement-destination-list summary').click();
    await movePanel.locator('.settler-destinations button').first().click();
    await movePanel.getByRole('button', { name: /^Disembark at/ }).click();
    const disembarked = JSON.parse(engine.tryMove(JSON.stringify(movement), JSON.stringify(sent.at(-1)), 0));
    assert.ok(
      disembarked.players[0].units.some((u) => u.id === 1 && u.position !== 'D2'),
      'Selected passenger reaches land',
    );
    assert.ok(
      disembarked.players[0].units.find((u) => u.id === 7).carried_units.some((u) => u.id === 2),
      'Other passenger stays aboard',
    );
    assert.deepEqual(errors, []);
    console.log(
      `Unit picker ${width}×${height}: casualties across hexes, limits, passenger movement and unobstructed controls`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
