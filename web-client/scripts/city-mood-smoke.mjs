// City mood faces must stay visible while card effects, incidents and combat are resolved.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'city-mood', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const cities = (game) =>
  game.players.flatMap((p) => (p.cities ?? []).map((c) => [c.position, c.mood_state.toLowerCase()]));
const scenarios = [
  // A position choice whose result is a mood decrease.
  ['incidents/famine/pestilence.outcome1', 'Select a city to decrease the mood'],
  // Unit choices (Pandemics may instead be paid with mood tokens).
  ['incidents/pandemics/pandemics.outcome', 'Select units to lose'],
  ['combat/remove_casualties_attacker.outcome', 'Remove 2 attacking units'],
];
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
const frames = (page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
async function visibleMoods(page) {
  return page.locator('.city-map-label').evaluateAll((labels) =>
    labels
      .filter((label) => {
        const box = label.getBoundingClientRect();
        return getComputedStyle(label).display !== 'none' && box.width > 0 && box.height > 0;
      })
      .map((label) => ({
        mood: label.dataset.mood,
        name: label.querySelector('.city-map-name')?.textContent ?? '',
        pointerEvents: getComputedStyle(label).pointerEvents,
      })),
  );
}
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 844],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
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
    await page.evaluate(() => {
      window.host = clash3d.launch('#app');
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false });
    });
    for (const [name, description] of scenarios) {
      const game = await fixture(name);
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(game), 0));
      await page.getByText(description).first().waitFor();
      await page.waitForTimeout(400);
      await frames(page);
      const shown = await visibleMoods(page);
      await page.screenshot({ path: `/tmp/clash-city-mood-${name.split('/').pop()}-${width}.png` });
      assert.equal(shown.length, cities(game).length, `${width} ${name}: every city shows its mood`);
      assert.deepEqual(
        shown.map((s) => s.mood).sort(),
        cities(game)
          .map(([, mood]) => mood)
          .sort(),
      );
    }

    // Unit choices: faces stay visible but never intercept a click meant for a unit model.
    const pandemics = await fixture('incidents/pandemics/pandemics.outcome');
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(pandemics), 0),
    );
    await page.getByText('Select units to lose').first().waitFor();
    await frames(page);
    assert.ok((await visibleMoods(page)).every((s) => s.pointerEvents === 'none'));

    // Resolving Pestilence lowers the chosen city's mood; the face updates with the engine result.
    const pestilence = await fixture('incidents/famine/pestilence.outcome1');
    const after = JSON.parse(
      engine.tryMove(
        JSON.stringify(pestilence),
        JSON.stringify({ Response: { SelectPositions: ['A1'] } }),
        0,
      ),
    );
    const before = Object.fromEntries(cities(pestilence));
    const changed = Object.fromEntries(cities(after));
    assert.notEqual(before.A1, changed.A1, 'Pestilence changes the mood of A1');
    await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(after), 0));
    await page.waitForTimeout(400);
    await frames(page);
    const moods = await visibleMoods(page);
    assert.ok(
      moods.some((s) => s.name.includes('A1') && s.mood === changed.A1),
      `${width}: A1 shows ${changed.A1} after Pestilence`,
    );

    // Strategy view shows the same moods inside its city summaries.
    await page.evaluate(() => host.emit('preferences', { sound: false, mapView: 'strategy' }));
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(pestilence), 0),
    );
    await page.locator('.strategy-map-tile').first().waitFor();
    await frames(page);
    for (const [position, mood] of cities(pestilence)) {
      const face = page.locator(`.strategy-map-tile[data-position="${position}"] .strategy-mood`);
      assert.equal(await face.getAttribute('data-mood'), mood, `${width}: Strategy ${position} mood`);
      assert.match(
        await page.locator(`.strategy-map-tile[data-position="${position}"]`).getAttribute('aria-label'),
        new RegExp(mood, 'i'),
      );
    }
    await page.screenshot({ path: `/tmp/clash-city-mood-strategy-${width}.png` });
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log('city mood smoke passed');
} finally {
  await browser.close();
}
