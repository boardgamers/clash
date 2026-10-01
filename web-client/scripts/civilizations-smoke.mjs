import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const ser = (g) => (typeof g === 'string' ? g : JSON.stringify(g)),
  seat = (g) => engine.currentPlayer(ser(g));
const initial = await engine.init(
  2,
  [],
  { civilization: 'ChooseCivilization' },
  'all-civs-ui',
  {},
);
const advance = (state, action) =>
  engine.tryMove(ser(state), typeof action === 'string' ? action : JSON.stringify(action), seat(state));
async function fixture(civ) {
  let s = initial;
  for (const c of [civ, 'Rome']) s = advance(s, { ChooseCivilization: c });
  const g = JSON.parse(s),
    p = g.players[seat(g)];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 8, culture_tokens: 8 };
  p.resource_limit.food = 7;
  return g;
}
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/') {
      res.setHeader('Content-Type', 'text/html');
      res.end(
        '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
      );
    } else {
      assert(['viewer.js', 'server_bg.wasm'].includes(req.url.slice(1)));
      res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
      res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
    }
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
async function load(width, dark, game) {
  const page = await browser.newPage({ viewport: { width, height: 900 } }),
    errors = [];
  let state = ser(game),
    moves = 0;
  page.on('pageerror', (e) => {
    errors.push(e.message);
    console.error('BROWSER ERROR', e.stack);
  });
  page.on('console', (m) => {
    if (m.type() === 'error') console.error('BROWSER CONSOLE', m.text());
  });
  page.on('requestfailed', (r) => console.error('REQUEST FAILED', r.url(), r.failure()));
  await page.exposeFunction('applyGameMove', async (action) => {
    state = advance(state, action);
    moves++;
    await page.evaluate(
      ({ state, seat }) => {
        host.emit('player', { index: seat });
        host.emit('state', state);
      },
      { state: engine.stripSecret(state, seat(state)), seat: seat(state) },
    );
  });
  await page.goto('http://127.0.0.1:' + server.address().port);
  await page.evaluate(
    ({ state, seat, dark }) => {
      window.host = clash3d.launch('#app');
      window.viewerReady = false;
      host.on('ready', () => (window.viewerReady = true));
      host.on('move', (move) => window.applyGameMove(move));
      host.emit('player', { index: seat });
      host.emit('preferences', { sound: false });
      host.emit('theme', { dark });
      host.emit('state', state);
    },
    { state: engine.stripSecret(state, seat(state)), seat: seat(state), dark },
  );
  await page.waitForFunction(() => window.viewerReady);
  return { page, errors, state: () => JSON.parse(state), moves: () => moves };
}
try {
  for (const [width, dark] of [
    [390, true],
    [1400, false],
  ]) {
    let app = await load(width, dark, initial),
      page = app.page;
    const picker = page.getByRole('dialog', { name: 'Choose civilization' });
    await picker.waitFor();
    assert.equal(
      await picker.getByRole('group', { name: 'Available civilizations' }).getByRole('button').count(),
      15,
    );
    for (const name of [
      'Aztecs',
      'Carthage',
      'Celts',
      'Egypt',
      'Huns',
      'Japan',
      'Maya',
      'Persia',
      'Phoenicia',
    ]) {
      await picker.getByRole('button', { name, exact: true }).click();
      await picker.getByRole('tab', { name: 'Advances' }).click();
      assert.equal(await picker.locator('article').count(), 4);
      await picker.getByRole('tab', { name: 'Leaders' }).click();
      assert.equal(await picker.locator('article').count(), 3);
    }
    await picker.getByRole('button', { name: 'Aztecs', exact: true }).click();
    await picker.getByRole('tab', { name: 'Advances' }).click();
    await picker.screenshot({ path: '/tmp/clash-all-civs-picker-' + width + '.png' });
    assert(await picker.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.deepEqual(app.errors, []);
    await page.close();
    const az = await fixture('Aztecs'),
      ap = az.players[seat(az)],
      enemy = 1 - seat(az);
    ap.advances.push('Tactics', 'Myths', 'Rituals');
    ap.resources.captives = 2;
    ap.captives = [0, 1].map((id) => ({ id, unit_type: 'Infantry', owner: enemy }));
    az.players[enemy].held_units = { infantry: 2 };
    app = await load(width, dark, az);
    page = app.page;
    await page.getByRole('button', { name: 'Abilities and cultural influence', exact: true }).click();
    await page.getByRole('button', { name: /Use Human Sacrifice/ }).click();
    const decision = page.getByRole('region', { name: 'Human Sacrifice', exact: true });
    await decision.waitFor();
    assert.equal(await page.getByRole('region', { name: 'Your resources' }).locator('.resource').count(), 8);
    await decision.screenshot({ path: '/tmp/clash-captive-choice-' + width + '.png' });
    assert(await decision.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await decision.locator('.decision-options button').first().click();
    await decision.getByRole('button', { name: 'Confirm', exact: true }).click();
    await page.getByRole('button', { name: /1 mood/i }).waitFor();
    await page.screenshot({ path: '/tmp/clash-captive-reward-' + width + '.png' });
    await page.getByRole('button', { name: /1 mood/i }).click();
    await page.waitForFunction(() => !document.querySelector('.decision-panel'));
    assert.equal(app.state().players[seat(az)].resources.mood_tokens, 9);
    assert.equal(app.moves(), 3);
    assert.deepEqual(app.errors, []);
    await page.close();
    const car = await fixture('Carthage'),
      cp = car.players[seat(car)];
    cp.advances.push('Husbandry');
    app = await load(width, dark, car);
    page = app.page;
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    const city = page.getByRole('dialog');
    await city.getByRole('button', { name: 'Recruit', exact: true }).click();
    await city.getByRole('button', { name: 'Add Elephant', exact: true }).click();
    const payment = city.getByLabel('Recruitment payment', { exact: true });
    await payment.waitFor();
    assert((await payment.locator('option').count()) > 1);
    const options = await payment
      .locator('option')
      .evaluateAll((els) => els.map((e) => ({ value: e.value, label: e.textContent })));
    const culture = options.find((o) => JSON.parse(o.value).culture_tokens === 2);
    assert(culture);
    await payment.selectOption(culture.value);
    await city.screenshot({ path: '/tmp/clash-carthage-recruit-' + width + '.png' });
    assert(await city.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await city.getByRole('button', { name: 'Recruit 1 unit', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.city-dialog'));
    assert(app.state().players[seat(car)].units.some((u) => u.unit_type === 'Elephant'));
    assert.equal(app.state().players[seat(car)].resources.culture_tokens, 6);
    assert.deepEqual(app.errors, []);
    await page.close();
    cp.advances.push('Fishing', 'Navigation', 'Tactics');
    cp.action_cards = [];
    for (const tile of car.map.tiles) tile[1] = 'Water';
    cp.units = [
      {
        id: 0,
        position: 'D3',
        unit_type: 'Ship',
        carried_units: [
          { id: 1, unit_type: 'Settler' },
          { id: 2, unit_type: 'Infantry' },
        ],
      },
      {
        id: 3,
        position: 'D3',
        unit_type: 'Ship',
        pirate: true,
        carried_units: [
          { id: 4, unit_type: 'Infantry' },
          { id: 5, unit_type: 'Cavalry' },
        ],
      },
    ];
    cp.next_unit_id = 6;
    const pirates = car.players.find((p) => p.civilization === 'Pirates');
    pirates.units = [];
    pirates.held_units = { ships: 1 };
    car.dice_roll_outcomes = [2, 11];
    let battle = JSON.parse(
      advance(car, {
        Movement: { Move: { units: [0], destination: 'D3', attack_pirates: true, payment: {} } },
      }),
    );
    while (battle.events?.at(-1)?.handler?.request?.ResourceReward) {
      battle = JSON.parse(
        advance(battle, {
          Response: { ResourceReward: battle.events.at(-1).handler.request.ResourceReward.reward.default },
        }),
      );
    }
    app = await load(width, dark, battle);
    page = app.page;
    const casualties = page.locator('.decision-panel');
    await casualties.waitFor();
    await casualties.getByRole('button', { name: /Settler/i }).click();
    await casualties.getByRole('button', { name: /Infantry/i }).click();
    await casualties.screenshot({ path: '/tmp/clash-pirate-passengers-' + width + '.png' });
    assert(await casualties.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await casualties.getByRole('button', { name: 'Confirm', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.decision-panel'));
    assert.deepEqual(
      app
        .state()
        .players[cp.id].units[0].carried_units.map((u) => u.id)
        .sort(),
      [4, 5],
    );
    assert.equal(app.moves(), 1);
    assert.deepEqual(app.errors, []);
    await page.close();
    console.log(
      width +
        'px: all 15 factions, captive exchange, recruitment payments, and pirate passenger choices passed',
    );
  }
} finally {
  await browser.close();
  await new Promise((r) => server.close(r));
}
