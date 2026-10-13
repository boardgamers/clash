// Influence: destination/building, source, then explicit method/payment confirmation.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'map-targets', {})).players.slice(2);
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
const frame = (page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
async function open(width, height, state, strategy = false) {
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
    ({ state, strategy }) => {
      window.host = clash3d.launch('#app');
      host.on('move', (m) => window.captureMove(m));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false, mapView: strategy ? 'strategy' : '3d' });
      host.emit('state', state);
    },
    { state: engine.stripSecret(JSON.stringify(state), 0), strategy },
  );
  return { page, errors, sent };
}
async function center(page, position) {
  const target = page.locator(`.map-hit-target[data-position="${position}"]`);
  await target.waitFor({ state: 'attached' });
  await page.waitForTimeout(250);
  await frame(page);
  const box = await target.boundingBox();
  assert.ok(box, `${position} is highlighted on the board`);
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}
/** Click around a hex until a click resolves to an exact building model. */
async function clickBuilding(page, position, done) {
  const { x, y } = await center(page, position);
  for (let r = 0; r <= 40; r += 5)
    for (let a = 0; a < 360; a += r ? 30 : 360) {
      await page.mouse.click(x + r * Math.cos((a * Math.PI) / 180), y + r * Math.sin((a * Math.PI) / 180));
      await frame(page);
      if (await done()) return;
    }
  assert.fail(`no building model at ${position} could be clicked`);
}
try {
  for (const [width, height, strategy] of [
    [1400, 900, false],
    [390, 844, false],
    [1400, 900, true],
  ]) {
    const label = `${width}px ${strategy ? 'strategy' : '3D'}`;
    // Cultural influence: the ability opens a map mode with highlighted target cities.
    const influence = await fixture('base/cultural_influence_instant');
    influence.current_player_index = 0;
    influence.players[1].cities[0].city_pieces = { temple: 1, academy: 1 };
    influence.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
    influence.players[0].resources.culture_tokens = 7;
    if (!strategy) influence.players[0].advances.push('Arts');
    const { page, errors, sent } = await open(width, height, influence, strategy);
    await page.getByRole('button', { name: 'Cultural influence', exact: true }).click();
    const panel = page.getByRole('region', { name: 'Cultural influence', exact: true });

    await panel.locator('.influence-picker').waitFor();
    assert.equal(await panel.locator('.influence-target-group').count(), 0, 'first step is map-only');
    assert.equal(await page.locator('.map-hit-target[data-position="C1"]').count(), 1);
    await page.locator('.map-hit-target[data-position="C1"]').press('Enter');
    await frame(page);
    assert.equal(
      await panel.locator('.influence-target-group').count(),
      1,
      `${label}: C1 opens its buildings`,
    );
    if (!strategy)
      await clickBuilding(page, 'C1', async () => (await panel.locator('.influence-preview').count()) > 0);
    else await panel.getByRole('button', { name: /Temple/ }).click();
    const route = await panel.locator('.influence-route').innerText();
    assert.match(route, /A1/);
    assert.match(route, /(Temple|Academy) at C1/);
    assert.equal(
      await page.locator('.map-hit-target[data-position="A1"]').getAttribute('aria-pressed'),
      'true',
    );
    await page.screenshot({ path: `/tmp/clash-influence-map-${width}-${strategy ? 'strategy' : '3d'}.png` });
    assert.deepEqual(sent, []);
    await panel.getByRole('button', { name: 'Choose payment' }).click();
    const methods = panel.getByLabel('Influence payment method');
    await methods.waitFor();
    assert.equal(await methods.getByRole('button').count(), strategy ? 1 : 2);
    assert.match(await methods.innerText(), /Maximum roll boost: \+4/);
    // Desktop Undo and the mobile panel Back share the same local navigation.
    const back = width < 600 ? panel.getByRole('button', { name: 'Previous influence step' }) : page.getByRole('button', { name: 'Undo last action', exact: true });
    await back.click();
    await panel.getByRole('button', { name: 'Choose payment' }).waitFor();
    assert.equal(await methods.count(), 0);
    await back.click();
    await panel.locator('.influence-target-group').waitFor();
    await back.click();
    assert.equal(await panel.locator('.influence-target-group').count(), 0);
    assert.deepEqual(sent, [], 'Undo during setup is local');
    await page.locator('.map-hit-target[data-position="C1"]').press('Enter');
    await panel.getByRole('button', { name: 'Temple', exact: true }).click();
    await panel.getByRole('button', { name: 'Choose payment' }).click();
    const confirm = methods.getByRole('button', { name: strategy ? /^Influence/ : /^Arts/ });
    const confirmBounds = await confirm.boundingBox();
    const panelBounds = await panel.boundingBox();
    const actionCost = confirm.locator('.influence-action-cost');
    assert.equal(await actionCost.getAttribute('aria-label'), strategy ? 'Costs 1 action' : 'Free action');
    const layout = await confirm.evaluate(button => {
      const title = button.querySelector('.method-title').getBoundingClientRect();
      const boost = button.querySelector(':scope > small').getBoundingClientRect();
      const action = button.querySelector('.influence-action-cost');
      return { titleY: title.y + title.height / 2, boostY: boost.y + boost.height / 2,
        actionColor: getComputedStyle(action).color, buttonColor: getComputedStyle(button).color };
    });
    assert.equal(layout.actionColor, layout.buttonColor, 'action cost inherits button contrast');
    if (width >= 700) assert.ok(Math.abs(layout.titleY - layout.boostY) < 2, 'desktop label, costs and boost share a row');
    assert.ok(confirmBounds && panelBounds && confirmBounds.y >= panelBounds.y && confirmBounds.y + confirmBounds.height <= panelBounds.y + panelBounds.height, 'roll confirmation stays visible within the panel');
    await page.screenshot({ path: `/tmp/clash-influence-payment-${width}-${strategy ? 'strategy' : '3d'}.png` });
    assert.deepEqual(sent, [], 'source confirmation does not spend or roll');
    await confirm.click();
    await page.waitForTimeout(100);
    assert.ok(sent.length >= 1);
    assert.equal(sent[0].Playing.InfluenceCultureAttempt.selected_structure.position, 'C1');
    assert.deepEqual(errors, []);
    console.log(`${label}: influence target ${route.replace(/\s+/g, ' ')} chosen on the map, then rolled.`);
    await page.close();
  }
} finally {
  await browser.close();
}
