import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View, Move, Decision } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const initial = () => engine.init(2, [], { undo: 'SamePlayer', civilization: 'Random' }, 'full-playtest', {});
const view = (state: string, seat = engine.currentPlayer(state)): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
const query = (state: string, input: unknown, seat = engine.currentPlayer(state)) =>
  JSON.parse(engine.webQuery(engine.stripSecret(state, seat), seat, JSON.stringify(input)));
const execute = (state: string, action: Move) =>
  engine.tryMove(state, JSON.stringify(action), engine.currentPlayer(state));

test('civilization selection opens a playable four-player game', async () => {
  let state = await engine.init(
    4,
    [],
    { undo: 'SamePlayer', civilization: 'ChooseCivilization' },
    'choose-civilizations',
    {},
  );
  for (let i = 0; i < 4; i++) {
    const current = view(state);
    assert.equal(current.civilizations!.length, 4 - i);
    state = execute(state, current.civilizations![0].action);
  }
  const current = view(state);
  assert.ok(current.canPlay);
  assert.equal(new Set(current.players.map((p) => p.civilization)).size, 4);
  assert.ok(current.players.every((p) => p.cities.some((c) => c.position === p.capital)));
});
async function fixture(name: string) {
  const s = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  // Small rule fixtures omit neutral factions, which influence offers also inspect.
  const base = JSON.parse(await initial());
  for (const npc of base.players.filter((p: any) => ['Barbarians', 'Pirates'].includes(p.civilization))) {
    if (!s.players.some((p: any) => p.civilization === npc.civilization))
      s.players.push({ ...npc, id: s.players.length });
  }
  return JSON.stringify(s);
}
function choose(
  state: string,
  d: Decision,
  values = d.options.slice(0, d.min).map((o) => o.value),
  payments = d.fields.map((f) => (f.optional ? {} : f.initial)),
) {
  return query(state, { kind: 'decision', values, payments }).action;
}

test('all six ages finish using only UI offers, including free research and incidents', async () => {
  let state = await initial();
  let decisions = 0;
  for (let step = 0; step < 180; step++) {
    if (JSON.parse(state).state === 'Finished') {
      assert.ok(decisions >= 25);
      return;
    }
    const v = view(state);
    assert.ok(v.supportedPhase);
    let move: Move;
    if (v.decision) {
      decisions++;
      move = choose(state, v.decision);
    } else if (v.choiceDecision) move = v.choiceDecision.choices.at(-1)!.action;
    else if (v.objectiveDecision) move = v.objectiveDecision.skip ?? v.objectiveDecision.cards[0].action;
    else {
      assert.ok(v.canEndTurn);
      move = { Playing: 'EndTurn' };
    }
    state = execute(state, move);
  }
  assert.fail('The age sequence did not finish');
});

test('mixed rewards, casualty selection and government choices use validated engine responses', async () => {
  let reward = await fixture('advances/trade_routes_with_currency.outcome');
  const d = view(reward).decision!;
  assert.ok(d.reward);
  assert.throws(() => choose(reward, d, [], [{ food: 3 }]), /resources/);
  const action = choose(reward, d, [], [{ food: 1, gold: 1 }]);
  assert.deepEqual(action, { Response: { ResourceReward: { food: 1, gold: 1 } } });
  reward = execute(reward, action);
  assert.ok(view(reward).supportedPhase);
  let combat = await fixture('combat/remove_casualties_attacker.outcome');
  const c = view(combat).decision!;
  assert.equal(c.min, 2);
  assert.throws(() => choose(combat, c, [0]), /Select/);
  assert.throws(() => choose(combat, c, [0, 0]), /Select/);
  assert.throws(() => choose(combat, c, [0, 999]), /Select/);
  const response = choose(combat, c, [0, 1]);
  combat = execute(combat, response);
  assert.equal(JSON.parse(combat).players[0].units.filter((u: any) => u.id === 0 || u.id === 1).length, 0);
  let government = await fixture('status_phase/change_government');
  let g = view(government).decision!;
  government = execute(government, choose(government, g));
  g = view(government).decision!;
  assert.ok(g.fields.length);
  government = execute(
    government,
    choose(
      government,
      g,
      [],
      g.fields.map((f) => f.initial),
    ),
  );
  g = view(government).decision!;
  assert.ok(g.options.length >= 2);
  government = execute(government, choose(government, g));
  assert.ok(view(government).supportedPhase);
});

test('multi-city happiness and free collection/happiness preserve action costs', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  const p = raw.players[seat];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  p.advances = ['Farming', 'Mining', 'Voting', 'FreeEconomy'];
  p.cities[0].mood_state = 'Neutral';
  const pos = p.cities[0].position;
  const other = pos === 'D2' ? 'E2' : 'E7';
  p.cities.push({ ...p.cities[0], position: other, mood_state: 'Angry' });
  let state = JSON.stringify(raw);
  let v = view(state);
  const variant = v.happinessActions!.find((a) => a.free)!;
  assert.ok(variant);
  assert.throws(
    () => query(state, { kind: 'happiness', cities: [[pos, 2]], variant: variant.value }),
    /valid/,
  );
  const happiness = query(state, {
    kind: 'happiness',
    cities: [
      [pos, 1],
      [other, 2],
    ],
    variant: variant.value,
  });
  state = execute(state, happiness.action);
  assert.equal(JSON.parse(state).actions_left, 3);
  assert.ok(JSON.parse(state).players[seat].cities.every((c: any) => c.mood_state === 'Happy'));
  v = view(state);
  const free = v.collectActions!.find((a) => a.free)!;
  assert.ok(free);
  const city = v.cities[0];
  const collected = query(state, {
    kind: 'collect',
    city: city.position,
    variant: free.value,
    selections: [{ ...city.choices[0], times: 1 }],
  });
  state = execute(state, collected.action);
  assert.equal(JSON.parse(state).actions_left, 3);
  assert.equal(view(state).collectActions!.length, 0, 'Free Economy is the only collection that turn');
});

test('leaders and supply replacements can be recruited through the same validated selection', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  const p = raw.players[seat];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  let state = JSON.stringify(raw);
  const v = view(state);
  const city = v.cities[0].position;
  const leader = v.cityActions[0].leaders![0].id;
  const recruited = query(state, { kind: 'recruit', city, units: { leader }, replaced: [] });
  state = execute(state, recruited.action);
  assert.ok(JSON.parse(state).players[seat].units.some((u: any) => u.unit_type.Leader === leader));
  const replacement = JSON.parse(await initial());
  const q = replacement.players[engine.currentPlayer(JSON.stringify(replacement))];
  q.resources = { food: 7, ore: 7, mood_tokens: 7, culture_tokens: 7 };
  const limit = view(JSON.stringify(replacement)).cityActions[0].recruits.find(
    (r) => r.type === 'Settler',
  )!.limit!;
  q.units = Array.from({ length: limit }, (_, id) => ({
    id,
    position: q.cities[0].position,
    unit_type: 'Settler',
  }));
  state = JSON.stringify(replacement);
  assert.throws(
    () => query(state, { kind: 'recruit', city: q.cities[0].position, units: { settlers: 1 }, replaced: [] }),
    /replacement/,
  );
  const offer = query(state, {
    kind: 'recruit',
    city: q.cities[0].position,
    units: { settlers: 1 },
    replaced: [0],
  });
  state = execute(state, offer.action);
  assert.equal(JSON.parse(state).players[q.id].units.length, limit);
});

test('ship movement, embarking and disembarking are executable and reject foreign or duplicate units', async () => {
  const sea = await fixture('movement/ship_navigation_unit_test');
  const destinations = query(sea, { kind: 'movement', units: [1] }).destinations;
  assert.ok(destinations.length > 1);
  execute(sea, destinations[0].action);
  assert.throws(() => query(sea, { kind: 'movement', units: [999] }), /units/);
  assert.throws(() => query(sea, { kind: 'movement', units: [1, 1] }), /units/);
  assert.throws(() => query(sea, { kind: 'movement', units: [1] }, 0), /decision/);
  for (const name of ['ship_embark', 'ship_disembark']) {
    const state = await fixture(`movement/${name}`);
    const v = view(state);
    const offers = v.units!.flatMap((u) => query(state, { kind: 'movement', units: [u.id] }).destinations);
    const destination = offers.find((d) =>
      name === 'ship_embark' ? d.carrier !== null : d.terrain !== 'Water',
    );
    assert.ok(destination, `${name} offers a legal destination`);
    execute(state, destination.action);
  }
});

test('wonder construction and action cards resolve through the new choice controls', async () => {
  for (const fixtureName of ['wonders/pyramids', 'action_cards/advance']) {
    let state = await fixture(fixtureName);
    let v = view(state);
    const offer = fixtureName.startsWith('wonders')
      ? v.wonderCards.find((c) => c.action)
      : v.actionCards!.find((c) => c.action);
    assert.ok(offer?.action, fixtureName);
    state = execute(state, offer.action);
    let count = 0;
    while (count++ < 15) {
      v = view(state);
      if (v.decision) state = execute(state, choose(state, v.decision));
      else if (v.choiceDecision) state = execute(state, v.choiceDecision.choices[0].action);
      else if (v.objectiveDecision)
        state = execute(state, v.objectiveDecision.skip ?? v.objectiveDecision.cards[0].action);
      else break;
    }
    assert.ok(count < 15);
    assert.ok(v.canPlay);
    if (fixtureName.startsWith('wonders'))
      assert.ok(JSON.parse(state).players.some((p: any) => p.wonders_built?.includes('Pyramids')));
  }
});

test('cultural influence, tactics cards and civilization abilities resolve engine requests', async () => {
  for (const name of [
    'base/cultural_influence',
    'advances/bartering',
    'advances/taxes',
    'tactics_cards/encircled.outcome',
    'civilizations/vikings/ship_construction',
  ]) {
    let state = await fixture(name);
    let v = view(state);
    if (name === 'base/cultural_influence') {
      assert.ok(v.influence?.length);
      state = execute(state, v.influence[0].action);
    } else if (name.startsWith('advances/')) {
      const offer = v.specialActions?.find((a) => a.name.toLowerCase() === name.split('/')[1]);
      assert.ok(offer, name);
      state = execute(state, offer.action);
    } else if (name.includes('ship_construction')) {
      const destination = v
        .units!.flatMap((u) => query(state, { kind: 'movement', units: [u.id] }).destinations)
        .find((d) => d.terrain !== 'Water' && d.carrier === null);
      assert.ok(destination);
      state = execute(state, destination.action);
    }
    let steps = 0;
    while (steps++ < 20) {
      v = view(state);
      if (v.decision) {
        const d = v.decision;
        // Exercise tactics-card selection rather than always declining it.
        const values = name.startsWith('tactics')
          ? d.options.slice(0, Math.max(1, d.min)).map((o) => o.value)
          : d.options.slice(0, d.min).map((o) => o.value);
        state = execute(state, choose(state, d, values));
      } else if (v.choiceDecision) state = execute(state, v.choiceDecision.choices.at(-1)!.action);
      else if (v.objectiveDecision)
        state = execute(state, v.objectiveDecision.skip ?? v.objectiveDecision.cards[0].action);
      else break;
    }
    assert.ok(steps < 20, name);
    assert.ok(v.supportedPhase);
  }
});

test('new hand and decision data remain private, and capital is independent of city ordering', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  const capital = view(JSON.stringify(raw), seat).players[seat].capital!;
  raw.players[seat].cities.unshift({ ...raw.players[seat].cities[0], position: 'E7' });
  let state = JSON.stringify(raw);
  let v = view(state, seat);
  assert.equal(v.cities.find((c) => c.position === capital)!.capital, true);
  assert.equal(v.cities[0].capital, false);
  assert.deepEqual(JSON.parse(engine.webView(engine.stripSecret(state, undefined), seat)).actionCards, []);
  state = await fixture('combat/remove_casualties_attacker.outcome');
  const active = engine.currentPlayer(state);
  assert.ok(view(state).decision);
  assert.equal(view(state, 1 - active).decision, null);
  assert.equal(
    JSON.parse(engine.webView(engine.stripSecret(state, undefined), undefined)).decision,
    undefined,
  );
  assert.throws(() => query(state, { kind: 'decision', values: [0, 1], payments: [] }, 1 - active), /turn/);
});
