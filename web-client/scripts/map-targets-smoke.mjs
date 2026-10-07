// Heavy Earthquake structures and cultural influence targets are chosen on the map.
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
    // Heavy Earthquake: choose 1–3 structures in your cities.
    let quake = await fixture('incidents/earthquake/earthquake');
    quake = JSON.parse(
      engine.tryMove(
        JSON.stringify(quake),
        JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
        0,
      ),
    );
    let { page, errors, sent } = await open(width, height, quake, strategy);
    const decision = page.locator('.decision-panel');
    await decision.locator('.structure-selection').waitFor();
    assert.deepEqual(
      (await page.locator('.map-hit-target').evaluateAll((els) => els.map((e) => e.dataset.position))).sort(),
      ['B2', 'B3', 'C1', 'C2'],
    );
    const pressed = () => decision.locator('.structure-options [aria-pressed=true]');
    // A city with a single eligible structure is chosen by clicking it.
    const b2 = await center(page, 'B2');
    await page.mouse.click(b2.x, b2.y);
    await frame(page);
    assert.equal(await pressed().count(), 1, `${label}: B2 city center chosen from the map`);
    assert.equal(
      await page.locator('.map-hit-target[data-position="B2"]').getAttribute('aria-pressed'),
      'true',
    );
    // C2 has four structures: the city opens its list; 3D building models are chosen exactly.
    // Keyboard users activate the highlighted hex itself.
    await page.locator('.map-hit-target[data-position="C2"]').press('Enter');
    await frame(page);
    assert.equal(
      await decision.locator('.structure-cities [aria-pressed=true]').innerText(),
      'C2',
      `${label}: clicking C2 opens its structures`,
    );
    assert.equal(await decision.locator('.structure-options button').count(), 4);
    assert.equal(await pressed().count(), 0, 'opening a city does not choose one of its structures');
    if (!strategy) {
      const before = await decision.locator('.structure-options [aria-pressed=true]').count();
      await clickBuilding(page, 'C2', async () => (await pressed().count()) > before);
      assert.equal(await pressed().count(), 1, `${label}: one C2 building chosen on the board`);
    } else await decision.getByRole('button', { name: 'Temple', exact: true }).click();
    const chosen = await pressed().getAttribute('aria-label');
    await page.screenshot({ path: `/tmp/clash-earthquake-map-${width}-${strategy ? 'strategy' : '3d'}.png` });
    const near = await center(page, 'C2');
    await page.mouse.move(5, height / 2);
    await frame(page);
    await page.screenshot({
      path: `/tmp/clash-earthquake-c2-${width}-${strategy ? 'strategy' : '3d'}.png`,
      clip: { x: near.x - 110, y: near.y - 110, width: 220, height: 220 },
    });
    assert.deepEqual(sent, [], `${label}: nothing is sent before confirmation`);
    await decision.getByRole('button', { name: /^Confirm/ }).click();
    await page.waitForTimeout(100);
    assert.equal(sent.length, 1);
    const structures = sent[0].Response.SelectStructures;
    assert.equal(structures.length, 2);
    assert.ok(structures.some((s) => s.position === 'B2' && s.structure === 'CityCenter'));
    assert.ok(structures.some((s) => s.position === 'C2'));
    assert.deepEqual(errors, []);
    console.log(`${label}: Earthquake chose B2 city center and C2 ${chosen} on the map, then confirmed.`);
    await page.close();

    // Cultural influence: the ability opens a map mode with highlighted target cities.
    const influence = await fixture('base/cultural_influence_instant');
    influence.current_player_index = 0;
    influence.players[1].cities[0].city_pieces = { temple: 1, academy: 1 };
    influence.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
    influence.players[0].resources.culture_tokens = 7;
    ({ page, errors, sent } = await open(width, height, influence, strategy));
    await page.getByRole('button', { name: 'Abilities and cultural influence', exact: true }).click();
    const panel = page.getByRole('region', { name: 'Abilities and influence', exact: true });
    assert.equal(await panel.locator('.influence-targets').count(), 0, 'the menu no longer lists targets');
    await panel.getByRole('button', { name: /Choose target/ }).click();
    await panel.locator('.influence-picker').waitFor();
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
    await panel.getByRole('button', { name: /Pay & roll|Roll for influence/ }).click();
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
