import { readableHistory } from './structured-log.ts';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';
import { collectionYield } from './collection-yield.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const serialize = (game: any) => (typeof game === 'string' ? game : JSON.stringify(game));
const seat = (game: any) => engine.currentPlayer(serialize(game));
const view = (game: any, player = seat(game)): View =>
  JSON.parse(engine.webView(engine.stripSecret(serialize(game), player), player));
const query = (game: any, input: unknown) =>
  JSON.parse(
    engine.webQuery(engine.stripSecret(serialize(game), seat(game)), seat(game), JSON.stringify(input)),
  );
const move = (game: any, action: unknown) =>
  JSON.parse(engine.tryMove(serialize(game), JSON.stringify(action), seat(game)));
async function fixture(name: string) {
  const game = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  const npcs = JSON.parse(
    await engine.init(2, [], { civilization: 'Random' }, 'fixture-npcs', {}),
  ).players.slice(2);
  game.players.push(...npcs);
  return game;
}

async function babylonia(leader?: string) {
  let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'babylonia-rules', {});
  state = engine.tryMove(
    state,
    JSON.stringify({ ChooseCivilization: 'Babylonia' }),
    engine.currentPlayer(state),
  );
  state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: 'Rome' }), engine.currentPlayer(state));
  const game = JSON.parse(state);
  const player = game.players[seat(game)];
  assert.equal(player.civilization, 'Babylonia');
  player.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 8, culture_tokens: 8 };
  player.resource_limit.food = 7;
  if (leader) {
    player.units.push({
      id: player.next_unit_id++,
      position: player.cities[0].position,
      unit_type: { Leader: leader },
    });
    player.available_leaders = player.available_leaders?.filter((l: string) => l !== leader);
  }
  return game;
}

test('faction setup previews four advances and three leaders and rejects unavailable choices', async () => {
  let game = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'faction-picker', {});
  const options = view(game).civilizations!;
  for (const civ of options) {
    assert.equal(civ.advances.length, 4);
    assert.equal(civ.leaders.length, 3);
    assert.ok(civ.advances.every((a) => a.name && a.description && a.requirement));
    assert.ok(civ.leaders.every((l) => l.abilities.length === 2));
  }
  for (const invalid of ['Pirates', 'Barbarians', 'Choose Civilization', 'Unknown'])
    assert.throws(() => move(game, { ChooseCivilization: invalid }), /playable civilization/);
  game = serialize(move(game, { ChooseCivilization: 'Babylonia' }));
  assert.ok(!view(game).civilizations!.some((c) => c.name === 'Babylonia'));
  assert.throws(() => move(game, { ChooseCivilization: 'Babylonia' }), /already chosen/);
});

test('Canals adds food only when the entire collection contains exactly one food', async () => {
  const game = await babylonia();
  const p = game.players[seat(game)];
  p.advances.push('Engineering');
  p.resources = {};
  const city = view(game).cities[0];
  const food = city.choices.find((c) => c.pile.food)!;
  const wood = city.choices.find((c) => c.pile.wood)!;
  assert.ok(food && wood);
  assert.deepEqual(collectionYield(food), { food: 2 }, 'Map and tile list include Canals');
  assert.deepEqual(collectionYield(wood), { wood: 1 });
  assert.deepEqual(collectionYield(food, [{ ...food, times: 2 }]), { food: 2 }, 'No bonus for repeated food');
  const collect = (selections: any[]) =>
    query(game, { kind: 'collect', city: city.position, variant: 'Collect', selections });
  const one = collect([{ ...food, times: 1 }]);
  assert.equal(one.total.food, 2);
  const mixed = collect([
    { ...food, times: 1 },
    { ...wood, times: 1 },
  ]);
  assert.equal(mixed.total.food, 2);
  assert.equal(mixed.total.wood, 1);
  const none = collect([{ ...wood, times: 1 }]);
  assert.equal(none.total.food ?? 0, 0);
  const after = move(game, mixed.action);
  assert.equal(after.players[seat(game)].resources.food, 2);
  assert.match(JSON.stringify(after.log), /Added 1 food.*Canals/);
  // Focused Collection permits two resources from one tile; that already meets 2 food.
  const raw = await fixture('civilizations/china/rice');
  raw.players[0].civilization = 'Babylonia';
  raw.players[0].advances.push('Engineering');
  raw.players[0].cities.find((c: any) => c.position === 'B3').mood_state = 'Happy';
  const other = view(raw).cities.find((c) => c.position === 'B3')!;
  const foods = other.choices.filter((c) => c.pile.food);
  assert.ok(foods.length >= 2);
  const selected = foods.slice(0, 2).map((c) => ({ ...c, times: 1 }));
  assert.deepEqual(
    selected.map((c) => collectionYield(c, selected)),
    [{ food: 1 }, { food: 1 }],
  );
  assert.deepEqual(
    collectionYield(selected[0], selected.slice(0, 1)),
    { food: 2 },
    'Removing a food tile restores Canals',
  );
  const two = query(raw, {
    kind: 'collect',
    city: 'B3',
    variant: 'Collect',
    selections: foods.slice(0, 2).map((c) => ({ ...c, times: 1 })),
  });
  assert.equal(two.total.food, 2);
});

test('Ziggurats preserves ideas while acquiring Dogma and accepts ideas for Temple costs', async () => {
  const game = await babylonia();
  const p = game.players[seat(game)];
  p.advances.push('Myths', 'StateReligion');
  p.incident_tokens = 3;
  const research = view(game).advances.find((a) => a.id === 'Dogma')!;
  assert.ok(research.action, research.reason ?? 'Dogma');
  const after = move(game, research.payments.find((p) => p.payment.food === 2)!.action);
  const player = after.players[seat(game)];
  assert.equal(player.resources.ideas, 7);
  assert.equal(player.resource_limit.ideas, 7);
  assert.ok(view(after).players[seat(game)].civilizationAdvances.find((a) => a.id === 'Ziggurats')!.active);
  player.resources = { ideas: 3 };
  player.cities.push({ position: 'E4', mood_state: 'Happy' });
  const temple = view(after).cityActions[0].buildings.find((b) => b.name === 'Temple')!;
  assert.deepEqual(temple.payment, { ideas: 2 }, 'State Religion removes the food cost before conversion');
  assert.ok(temple.choices[0]?.action, temple.reason ?? 'Temple');
  const built = move(after, temple.choices[0].action);
  assert.equal(built.players[seat(game)].cities[0].city_pieces.temple, seat(game));
  assert.equal(built.players[seat(game)].resources.ideas, 1);
});

test('Star Catalogues asks for its reward before revealing or removing an event from the deck', async () => {
  const game = await babylonia();
  const p = game.players[seat(game)];
  p.advances.push('Math');
  p.incident_tokens = 1;
  p.resources.ideas = 0;
  const beforeDeck = [...game.incidents_left];
  const astronomy = view(game).advances.find((a) => a.id === 'Astronomy')!;
  assert.ok(astronomy.action, astronomy.reason ?? 'Astronomy');
  const pending = move(game, astronomy.action);
  assert.deepEqual(pending.incidents_left, beforeDeck);
  assert.equal(view(pending).choiceDecision?.choices.length, 2);
  assert.match(JSON.stringify(pending.events), /StarCatalogues/);
  const after = move(pending, { Response: { ResourceReward: { ideas: 1 } } });
  assert.equal(after.incidents_left.length, beforeDeck.length - 1);
  assert.ok((after.players[seat(game)].resources.ideas ?? 0) >= 1);
  assert.equal(
    JSON.stringify(readableHistory(after).log).match(/Star Catalogues.*?Gain 1 ideas?/g)?.length,
    1,
  );
});

test('Hammurabi discounts only his Fortress and offers Lawgiver alongside normal happiness costs', async () => {
  const game = await babylonia('Hammurabi');
  const index = seat(game),
    p = game.players[index];
  const position = p.cities[0].position;
  p.advances.push('Tactics', 'Siegecraft', 'Voting');
  p.cities.push({ position: 'E4', mood_state: 'Neutral', city_pieces: { academy: index } });
  const fortress = view(game).cityActions[0].buildings.find((b) => b.name === 'Fortress')!;
  assert.deepEqual(fortress.payment, { ore: 1 });
  assert.deepEqual(view(game).cityActions[1].buildings.find((b) => b.name === 'Fortress')!.payment, {
    food: 1,
    wood: 1,
    ore: 1,
  });
  const built = move(game, fortress.choices[0].action);
  assert.equal(built.players[index].resources.ore, 6);
  assert.equal(built.players[index].resources.food, 7);
  p.cities[0].city_pieces = { fortress: index };
  p.cities[0].mood_state = 'Angry';
  p.resources = { culture_tokens: 1, mood_tokens: 3 };
  const offers = view(game).cityActions[0].happiness;
  assert.deepEqual(offers.find((o) => o.lawgiver)!.payment, { culture_tokens: 1 });
  const voting = view(game).happinessActions!.find((a) => a.name === 'Voting')!;
  const quoted = query(game, {
    kind: 'happiness',
    cities: [
      [position, 2],
      ['E4', 1],
    ],
    variant: voting.value,
    lawgiver: true,
  });
  assert.deepEqual(quoted.payment, { mood_tokens: 3, culture_tokens: 1 });
  const happy = move(game, quoted.action);
  assert.equal(happy.actions_left, game.actions_left);
  assert.deepEqual(happy.players[index].resources ?? {}, {});
  assert.ok(happy.players[index].cities.every((c: any) => c.mood_state === 'Happy'));
  assert.throws(
    () =>
      query(game, {
        kind: 'happiness',
        cities: [[position, 1]],
        variant: 'IncreaseHappiness',
        lawgiver: true,
      }),
    /makes.*happy/,
  );
  p.units = [];
  assert.throws(() => move(game, quoted.action), /Lawgiver needs/);
});

test('Code of Laws draws only after a successful influence attempt with at most four action cards', async () => {
  for (const count of [4, 5]) {
    const game = await fixture('base/cultural_influence_instant');
    game.players[1].civilization = 'Babylonia';
    game.players[1].advances.push('Writing');
    game.players[1].action_cards = Array.from({ length: count }, (_, i) => i + 1);
    game.action_cards_left = [10, 11, 12];
    const target = view(game).influence!.find((i) => i.action)!;
    assert.ok(target);
    let after = move(game, target.action);
    // This fixture requires range payment before rolling a successful influence.
    for (let i = 0; i < 3 && view(after).decision?.fields?.length; i++) {
      const decision = view(after).decision!;
      const action = query(after, {
        kind: 'decision',
        selections: [],
        payments: decision.fields.map((f) => f.initial),
      }).action;
      after = move(after, action);
    }
    assert.equal(after.players[1].action_cards.length, count === 4 ? 5 : 5);
    assert.equal(after.players[0].cities.find((c: any) => c.position === 'C2').city_pieces.fortress, 1);
  }
});

test('Nebuchadnezzar and Nabopolassar apply their combat bonuses only in qualifying battles', async () => {
  for (const [leader, temple, ownCities, bonus] of [
    ['Nebuchadnezzar', true, 1, 'Iconoclast'],
    ['Nebuchadnezzar', false, 1, null],
    ['Nabopolassar', false, 1, 'Liberator'],
    ['Nabopolassar', false, 2, null],
  ] as const) {
    const game = await fixture('civilizations/china/fast_war');
    const p = game.players[0];
    p.civilization = 'Babylonia';
    p.units[0].unit_type = { Leader: leader };
    if (ownCities === 2) p.cities.push({ position: 'D2', mood_state: 'Happy' });
    if (temple) game.players[1].cities[0].city_pieces = { temple: 1 };
    const after = move(game, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
    const log = JSON.stringify(after.log);
    if (bonus) assert.match(log, new RegExp(`${bonus} adds \\+2 combat value`));
    else assert.doesNotMatch(log, /(?:Iconoclast|Liberator) adds/);
  }
});

test('Revolt reinforces the leader after a city is captured, falling back to a city with space', async () => {
  for (const fullStack of [false, true]) {
    const game = await fixture('civilizations/china/fast_war');
    const p = game.players[1];
    p.civilization = 'Babylonia';
    p.cities.push({ position: 'D7', mood_state: 'Happy' });
    p.units.push({ id: p.next_unit_id++, position: 'D7', unit_type: { Leader: 'Nabopolassar' } });
    if (fullStack) {
      p.cities.push({ position: 'C7', mood_state: 'Happy' });
      for (let i = 0; i < 3; i++)
        p.units.push({ id: p.next_unit_id++, position: 'D7', unit_type: 'Infantry' });
    }
    const expectedId = p.next_unit_id;
    const after = move(game, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
    const reinforcement = after.players[1].units.find((u: any) => u.id === expectedId);
    assert.equal(reinforcement?.unit_type, 'Infantry');
    assert.equal(reinforcement.position, fullStack ? 'C7' : 'D7');
    assert.ok(!after.players[1].cities.some((c: any) => c.position === 'D8'));
    assert.match(JSON.stringify(after.log), /Revolt/);
  }
});

test('Nebuchadnezzar draws Great Gardens and discounts it only in his own city', async () => {
  const game = await babylonia('Nebuchadnezzar');
  const index = seat(game),
    p = game.players[index],
    position = p.cities[0].position;
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, culture_tokens: 10 };
  p.cities.push({ position: 'E4', mood_state: 'Happy' });
  const opponent = game.players[1 - index];
  opponent.wonder_cards = ['GreatGardens'];
  game.wonders_left = game.wonders_left.filter((w: string) => w !== 'GreatGardens');
  let after = move(game, view(game).advances.find((a) => a.id === 'Engineering')!.action);
  assert.ok(after.players[index].wonder_cards.includes('GreatGardens'));
  assert.ok(!after.players[1 - index].wonder_cards?.includes('GreatGardens'));
  assert.ok(Object.keys(after.players[1 - index].event_info).some((key) => /replacement/i.test(key)));
  after.players[index].advances.push('Irrigation');
  for (const city of [position, 'E4']) {
    let placement = move(after, { Playing: { WonderCard: 'GreatGardens' } });
    placement = move(placement, { Response: { SelectPositions: [city] } });
    const cost = view(placement).decision!.fields[0].initial;
    assert.equal(cost.culture_tokens, city === position ? 3 : 5);
    assert.equal(placement.actions_left, after.actions_left - (city === position ? 0 : 1));
    const built = move(placement, { Response: { Payment: [cost] } });
    assert.ok(
      built.players[index].cities
        .find((c: any) => c.position === city)
        .city_pieces.wonders.includes('GreatGardens'),
    );
  }
});
