// Run after `npm run build` in web-client:
//   node scripts/ashoka-influence-smoke.mjs
// Exercise Ashoka and Darius in the shipped viewer and local authoritative engine.
// Every HTTP request is intercepted; no live game, CDN, or development server is
// contacted. Screenshots are written only under /tmp.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const base = JSON.parse(await readFile(
  new URL('../../server/tests/test_games/base/cultural_influence_instant.json', import.meta.url),
  'utf8',
));
const fresh = JSON.parse(await engine.init(2, [], {}, 'ashoka-influence-smoke', {}));
const stripped = (state) => engine.stripSecret(JSON.stringify(state), 0);
const view = (state) => JSON.parse(engine.webView(stripped(state), 0));
const move = (state, action) => JSON.parse(engine.tryMove(
  JSON.stringify(state), JSON.stringify(action), 0,
));
const unity = [{ name: 'Cultural Unity', resources: { mood_tokens: 1 } }];

function fixture(leader) {
  const game = structuredClone(base);
  // Older saved fixtures omit NPCs required by the current engine.
  game.players = [...game.players.slice(0, 2), ...structuredClone(fresh.players.slice(2))];
  game.current_player_index = 0;
  // Stored outcome 10 is a die roll of 6. Ashoka first rolls raw 2 + Conversion.
  game.dice_roll_outcomes = leader === 'Ashoka' ? [10, 2] : [10];
  const player = game.players[0];
  player.civilization = leader === 'Ashoka' ? 'India' : 'Persia';
  player.advances.push('Arts', ...(leader === 'Ashoka'
    ? ['Myths', 'StateReligion', 'Dogma', 'Conversion']
    : ['Myths', 'Priesthood']));
  player.cities = ['A1', 'C2'].map((position) => ({
    position, mood_state: 'Happy', city_pieces: { temple: 0 },
  }));
  player.units = [{ id: 0, position: 'C2', unit_type: { Leader: leader } }];
  player.next_unit_id = 1;
  if (leader === 'Darius') player.resources.mood_tokens = 12; // The reward must not be capped at 7.
  game.players[1].cities[0].city_pieces = { market: 1 };

  const offers = view(game).influence.filter((offer) => offer.position === 'C1' && offer.name === 'Market');
  const arts = offers.find((offer) => offer.variant === 'Arts');
  assert(arts, `${leader}: the fixture must offer Arts against the Market at C1`);
  for (const offer of offers) {
    const ordinary = offer.origins.find((from) => from.position === 'A1');
    const special = offer.origins.find((from) => from.position === 'C2');
    assert(ordinary && special, `${leader}: both temple cities must be eligible sources`);
    assert.deepEqual(ordinary.payment, special.payment, `${leader}: both sources have the same range cost`);
    assert.deepEqual(ordinary.actionPayment, special.actionPayment, `${leader}: both sources have the same action fee`);
    assert.equal(ordinary.free, special.free, `${leader}: the source benefit must not cost an extra action`);
    assert.equal(ordinary.rollBonus, special.rollBonus, `${leader}: both sources have the same roll bonus`);
    assert.equal(ordinary.preventBoost, special.preventBoost, `${leader}: both sources have the same boostability`);
    assert.equal(ordinary.reroll, false, 'A1 has no Ashoka');
    assert.equal(special.reroll, leader === 'Ashoka', `${leader}: only Ashoka offers the Buddhism reroll`);
    assert.deepEqual(ordinary.successRewards, [], `${leader}: A1 offers no source success reward`);
    assert.deepEqual(special.successRewards, leader === 'Darius' ? unity : [],
      `${leader}: only Darius' exact city offers Cultural Unity`);
  }
  const selected = arts.origins.find((from) => from.position === 'C2');
  assert.equal(selected.rollBonus, leader === 'Ashoka' ? 1 : 0, 'Only Conversion adds one to the die');
  return { leader, game, selected };
}
const fixtures = ['Ashoka', 'Darius'].map(fixture);

const french = JSON.parse(await readFile(new URL('../dist/fr.json', import.meta.url), 'utf8'));
for (const key of [
  'Cultural influence', 'Change source', 'Choose payment', 'Influence payment method',
  'Arts', 'Reroll with Buddhism', 'Keep this roll', 'Cultural Unity', 'Mood',
]) {
  assert.equal(typeof french[key], 'string', `The shipped French catalog must contain ${key}`);
  assert(french[key], `The shipped French translation must not be empty: ${key}`);
}
for (const key of ['Reroll with Buddhism', 'Keep this roll', 'Cultural Unity']) {
  assert.notEqual(french[key], key);
}

const origin = 'http://clash.test';
const assets = new Map(await Promise.all([
  ['viewer.js', 'text/javascript'],
  ['server_bg.wasm', 'application/wasm'],
  ['fr.json', 'application/json'],
].map(async ([name, contentType]) => [
  `/${name}`,
  { contentType, body: await readFile(new URL(`../dist/${name}`, import.meta.url)) },
])));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
const frame = (page) => page.evaluate(() => new Promise((resolve) => {
  requestAnimationFrame(() => requestAnimationFrame(resolve));
}));
const sentMoves = (page) => page.evaluate(() => window.sentMoves);

async function smoke({ leader, game, selected }, width, height, locale) {
  const label = `${leader} ${width}px ${locale}`;
  const text = (key) => locale === 'fr' ? french[key] : key;
  const page = await browser.newPage({ viewport: { width, height }, serviceWorkers: 'block' });
  page.setDefaultTimeout(15000);
  const errors = [], unexpectedRequests = [];
  page.on('pageerror', (error) => errors.push(error.message));
  try {
    await page.route('**/*', async (route) => {
      const url = new URL(route.request().url());
      // Match the localization smokes: optional display fonts are stubbed,
      // not downloaded. All other requests must be one of our local assets.
      if (url.origin === 'https://fonts.googleapis.com' && url.pathname === '/css2') {
        return route.fulfill({ contentType: 'text/css', body: '' });
      }
      if (url.origin === origin && url.pathname === '/') {
        return route.fulfill({
          contentType: 'text/html',
          body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
        });
      }
      const asset = url.origin === origin && assets.get(url.pathname);
      if (asset) return route.fulfill(asset);
      unexpectedRequests.push(url.href);
      return route.abort();
    });
    await page.goto(`${origin}/`);
    await page.evaluate(({ state, locale }) => {
      window.host = clash3d.launch('#app');
      window.sentMoves = [];
      host.on('move', (action) => window.sentMoves.push(JSON.parse(action)));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false, locale });
      host.emit('state', state);
    }, { state: stripped(game), locale: locale === 'fr' ? 'fr-CA' : 'en' });
    await page.waitForSelector(`[data-clash-locale="${locale}"]`);
    await page.getByRole('button', { name: text('Cultural influence'), exact: true }).click();
    const target = page.locator('.map-hit-target[data-position="C1"]');
    await target.press('Enter');
    const changeSource = page.locator('.influence-picker').getByRole('button', {
      name: text('Change source'), exact: true,
    });
    const source = changeSource.locator('strong');
    await source.waitFor({ state: 'visible' });
    const assertReward = async (scope, expected, summary = false) => {
      await frame(page);
      const reward = scope.locator('.source-success-reward');
      assert.equal(await reward.count(), expected ? 1 : 0, `${label}: only the Darius source shows a reward`);
      if (expected) {
        assert(await reward.isVisible(), `${label}: Cultural Unity must be visible`);
        if (summary) {
          assert((await reward.innerText()).includes(text('Cultural Unity')),
            `${label}: the payment summary localizes Cultural Unity`);
        } else {
          assert.equal(await reward.getAttribute('title'), text('Cultural Unity'),
            `${label}: the source reward tooltip localizes Cultural Unity`);
        }
        assert.match(await reward.innerText(), /\+\s*1\b/, `${label}: the reward is +1`);
        assert.equal(await reward.getByLabel(`1 ${text('Mood')}`, { exact: true }).count(), 1,
          `${label}: the reward is one mood token, not a cost or another resource`);
      }
    };
    const assertSource = async (position, message) => {
      await frame(page);
      assert.equal(await source.innerText(), position, `${label}: ${message}`);
      const chosen = page.locator('.influence-origins button.selected');
      assert.equal((await chosen.innerText()).match(/A1|C2/)?.[0], position);
      await assertReward(chosen, leader === 'Darius' && position === 'C2');
      await assertReward(page.locator('.influence-origins button').filter({ hasText: 'A1' }), false);
      await assertReward(page.locator('.influence-origins button').filter({ hasText: 'C2' }), leader === 'Darius');
    };
    await assertSource('C2', `the equal-cost default prefers ${leader === 'Ashoka' ? 'Buddhism' : 'Cultural Unity'}`);

    // An ordinary manual choice is important: merely checking C2 could pass
    // even if selecting C1 discards the source and recalculates it.
    for (const position of ['A1', 'C2']) {
      await page.locator('.influence-origins button').filter({ hasText: position }).click();
      await assertSource(position, 'the source button respects the explicit choice');
      await target.press('Enter');
      await assertSource(position, 'reselecting the same target preserves the explicit source');
      if (leader === 'Darius' && position === 'A1') {
        await page.getByRole('button', { name: text('Choose payment'), exact: true }).click();
        await page.locator('.roll-summary').waitFor({ state: 'visible' });
        await assertReward(page.locator('.roll-summary'), false, true);
        assert.equal(await source.innerText(), 'A1', `${label}: payment keeps the non-reward source`);
        await changeSource.click();
        await assertSource('A1', 'returning from payment keeps the explicit non-reward source');
      }
    }
    assert.deepEqual(await sentMoves(page), [], `${label}: selection must not spend or roll`);

    // Do not inject a pre-selected attempt: submit through the actual
    // source/payment UI so the captured action proves C2 survived it.
    await page.getByRole('button', { name: text('Choose payment'), exact: true }).click();
    const methods = page.getByLabel(text('Influence payment method'), { exact: true });
    await methods.waitFor({ state: 'visible' });
    await assertReward(page.locator('.roll-summary'), leader === 'Darius', true);
    const unclipped = await methods.locator('button').evaluateAll(buttons => buttons.every(button => {
      const rect = button.getBoundingClientRect();
      return [...button.children].every(child => {
        const text = child.getBoundingClientRect();
        return text.top >= rect.top && text.bottom <= rect.bottom && text.left >= rect.left && text.right <= rect.right;
      });
    }));
    assert(unclipped, `${label}: payment labels and boost limits fit inside their buttons`);
    await page.screenshot({ path: `/tmp/clash-influence-${leader}-${locale}-${width}-payment.png` });
    assert.deepEqual(await sentMoves(page), [], `${label}: choosing payment is still local`);
    await methods.getByRole('button', { name: /^Arts\b/ }).click();
    await page.waitForFunction(() => window.sentMoves.length > 0);
    const submitted = await sentMoves(page);
    assert.equal(submitted.length, 1, `${label}: exactly one influence attempt is submitted`);
    const attempt = submitted[0].Playing?.InfluenceCultureAttempt;
    assert.equal(attempt?.starting_position, 'C2', `${label}: the real attempt starts at ${leader}`);
    assert.equal(attempt.selected_structure.position, 'C1');
    assert.deepEqual(attempt.selected_structure.structure, { Building: 'Market' });
    assert.deepEqual(submitted[0], selected.action, `${label}: the real action uses Arts from C2`);

    // Apply the captured attempt with the authoritative local engine. The
    // action fee and range payment are separate server continuations, not
    // additional rolls. Only Ashoka's resulting reroll state goes to the viewer.
    let state = move(game, submitted[0]);
    if (view(state).influenceContext?.stage === 'payment') {
      state = move(state, { Response: { Payment: [selected.actionPayment] } });
    }
    if (view(state).influenceContext?.stage === 'range') {
      state = move(state, { Response: { Payment: [selected.payment] } });
    }
    if (leader === 'Ashoka') {
      assert.deepEqual(view(state).influenceContext, {
        name: 'Market', source: 'C2', target: 'C1', stage: 'reroll',
        roll: 3, rollBonus: 1, threshold: 5,
      }, `${label}: raw die 2 plus Conversion must reach the Buddhism decision`);
      await page.evaluate((state) => host.emit('state', state), stripped(state));
      const reroll = page.getByRole('button', { name: text('Reroll with Buddhism'), exact: true });
      const keep = page.getByRole('button', { name: text('Keep this roll'), exact: true });
      await reroll.waitFor({ state: 'visible' });
      await keep.waitFor({ state: 'visible' });
      assert(await reroll.isEnabled(), `${label}: reroll must be actionable`);
      assert(await keep.isEnabled(), `${label}: keeping the roll must be actionable`);
      const flow = page.locator('.influence-flow');
      assert.equal(await flow.locator('.influence-roll strong').innerText(), '3');
      assert.match(await flow.locator('.influence-roll small').innerText(), /\+1/);
      assert.equal(await flow.locator('.influence-route b').first().innerText(), 'C2');
      assert.equal(await flow.locator('.influence-route b').last().innerText(), 'C1');
      await page.screenshot({ path: `/tmp/clash-influence-Ashoka-${locale}-${width}-reroll.png` });
      await reroll.click();
      await page.waitForFunction(() => window.sentMoves.length > 1);
      assert.deepEqual(await sentMoves(page), [submitted[0], { Response: { Bool: true } }],
        `${label}: the Buddhism button submits the affirmative response`);
      const resolved = move(state, { Response: { Bool: true } });
      assert.equal(resolved.successful_cultural_influence, true);
      assert.equal(resolved.players[0].event_info.Buddhism, 'used');
    } else {
      assert.equal(state.successful_cultural_influence, true, `${label}: the forced roll of 6 succeeds`);
      assert.equal(state.players[1].cities.find((city) => city.position === 'C1').city_pieces.market, 0,
        `${label}: the actual Market is influenced`);
      const moodPaid = (selected.actionPayment?.mood_tokens ?? 0) + (selected.payment?.mood_tokens ?? 0);
      assert.equal(game.players[0].resources.mood_tokens, 12);
      assert.equal(state.players[0].resources.mood_tokens, 12 - moodPaid + 1,
        `${label}: Cultural Unity awards exactly one mood token after fees and range payment`);
      assert(state.players[0].resources.mood_tokens > game.players[0].resource_limit.mood_tokens,
        `${label}: Cultural Unity is not capped at the ordinary resource limit`);
      assert.deepEqual(await sentMoves(page), submitted, `${label}: the preview never submits a reward action`);
    }
    await frame(page);
    assert.equal(await page.locator('[data-clash-locale-status]').count(), 0);
    assert.deepEqual(errors, [], `${label}: no page errors`);
    assert.deepEqual(unexpectedRequests, [], `${label}: no unexpected network requests`);
    console.log(`${label}: equal-cost default, manual source retention, Arts attempt from C2, ${leader === 'Ashoka'
      ? 'roll 3 and Buddhism reroll' : 'localized reward preview and uncapped +1 mood on success'} passed`);
  } finally {
    await page.close();
  }
}

try {
  for (const [width, height] of [[1400, 900], [390, 844]]) {
    for (const locale of ['en', 'fr']) {
      for (const scenario of fixtures) await smoke(scenario, width, height, locale);
    }
  }
} finally {
  await browser.close();
}
