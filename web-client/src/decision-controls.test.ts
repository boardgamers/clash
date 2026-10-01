import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import {
  researchDecision,
  mapDecisionOptions,
  mapDecisionIndex,
  toggleDecisionSelection,
} from './decision-controls.ts';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (name: string) =>
  readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8');
const view = (state: string, seat = engine.currentPlayer(state)): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));

test('Great Seer separates objective conditions and identifies who receives the unchosen cards', async () => {
  for (const players of [2, 3]) {
    const game = JSON.parse(await engine.init(players, [], {}, 'seer-choices', {}));
    const seat = engine.currentPlayer(JSON.stringify(game));
    game.players[seat].action_cards = [158];
    const before = game.players.map((p: { objective_cards: number[] }) => p.objective_cards);
    let state = engine.tryMove(JSON.stringify(game), JSON.stringify({ Playing: { ActionCard: 158 } }), seat);
    const decision = view(state).decision!;
    assert.equal(decision.options.length, players);
    assert.match(decision.description, /Choose your next objective card/);
    assert.match(
      decision.description,
      players === 2 ? /The other card goes to/ : /assign the remaining cards/,
    );
    assert.match(decision.description, /next objective draw/);
    for (const option of decision.options) {
      assert.equal(option.card?.kind, 'objective');
      if (option.card?.kind !== 'objective') continue;
      assert.equal(option.card.objectives.length, 2);
      assert.deepEqual(option.card.objectives.map((o) => o.name).join('/'), option.name);
      assert.ok(
        option.card.objectives.every((o) => o.description && ['Instant', 'Status phase'].includes(o.timing)),
      );
    }
    assert.ok(!view(state, (seat + 1) % players).decision);
    while (view(state).decision?.name === 'Great Seer') {
      const d = view(state).decision!;
      const { action } = JSON.parse(
        engine.webQuery(
          engine.stripSecret(state, seat),
          seat,
          JSON.stringify({ kind: 'decision', values: [d.options[0].value], payments: [] }),
        ),
      );
      state = engine.tryMove(state, JSON.stringify(action), seat);
    }
    const after = JSON.parse(state);
    assert.deepEqual(
      after.players.map((p: { objective_cards: number[] }) => p.objective_cards),
      before,
    );
    const assigned = after.permanent_effects.find((effect: { GreatSeer?: unknown }) => effect.GreatSeer)
      .GreatSeer.assigned_objectives;
    assert.equal(assigned.length, players);
    assert.equal(new Set(assigned.map((a: { player: number }) => a.player)).size, players);
    assert.equal(
      assigned[0].objective_card,
      (decision.options[0].value as { ObjectiveCard: number }).ObjectiveCard,
    );
  }
});

test('Spy card choices expose action and battle uses separately', () => {
  let state = engine.tryMove(fixture('action_cards/spy'), JSON.stringify({ Playing: { ActionCard: 7 } }), 0);
  state = engine.tryMove(state, JSON.stringify({ Response: { Payment: [{ culture_tokens: 1 }] } }), 0);
  const cards = view(state).decision!.options.filter((o) => o.card?.kind === 'action');
  assert.equal(view(state).decision!.tacticsSelection, false);
  assert.ok(cards.length > 0);
  for (const option of cards) {
    if (option.card?.kind !== 'action') continue;
    assert.ok(option.card.description);
    assert.ok(option.card.tactics?.description);
    assert.equal(option.name, `${option.card.name}/${option.card.tactics?.name}`);
  }
});

for (const seat of [0, 1]) {
  test(`combat card choices use only the battle face for the ${seat ? 'defender' : 'attacker'}`, () => {
    const game = JSON.parse(fixture('tactics_cards/peltasts'));
    game.players[0].action_cards = [];
    game.players[1].action_cards = [];
    game.players[seat].action_cards = [4]; // Inspiration / Peltasts
    game.players[seat].resources.culture_tokens = 0;
    const state = engine.tryMove(
      JSON.stringify(game),
      JSON.stringify({ Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }),
      0,
    );
    const d = view(state, seat).decision!;
    assert.equal(d.tacticsSelection, true);
    assert.equal(d.options[0].card?.kind, 'action');
    const option = d.options[0];
    if (option.card?.kind !== 'action') throw new Error('Expected action card');
    assert.equal(option.card.name, 'Inspiration');
    assert.equal(option.card.tactics?.name, 'Peltasts');
    for (const play of [true, false]) {
      const { action } = JSON.parse(
        engine.webQuery(
          engine.stripSecret(state, seat),
          seat,
          JSON.stringify({ kind: 'decision', values: play ? [option.value] : [], payments: [] }),
        ),
      );
      const after = JSON.parse(engine.tryMove(state, JSON.stringify(action), seat));
      assert.deepEqual(
        after.players[seat].advances,
        game.players[seat].advances,
        'Civil research effect must not trigger',
      );
      assert.equal(after.players[seat].resources.culture_tokens ?? 0, 0);
      assert.deepEqual(after.players[seat].action_cards ?? [], play ? [] : [4]);
    }
  });
}

test('free research exposes exactly the legal choices without paid actions or resource requirements', () => {
  const raw = JSON.parse(fixture('status_phase/free_advance.outcome'));
  const seat = engine.currentPlayer(JSON.stringify(raw));
  raw.players[seat].resources = {};
  const state = JSON.stringify(raw);
  const v = view(state);
  assert.ok(researchDecision(v));
  assert.deepEqual(
    v.advances
      .filter((a) => a.action)
      .map((a) => a.id)
      .sort(),
    v.decision!.options.map((o) => o.value).sort(),
  );
  const storage = v.advances.find((a) => a.id === 'Storage')!;
  assert.deepEqual(storage.action, { Response: { SelectAdvance: 'Storage' } });
  assert.equal(storage.costAmount, 0);
  assert.deepEqual(storage.payments, []);
  const next = JSON.parse(engine.tryMove(state, JSON.stringify(storage.action), seat));
  assert.ok(next.players[seat].advances.includes('Storage'));
  assert.equal(next.actions_left, raw.actions_left);
  assert.equal(next.players[seat].resources.food ?? 0, 0);
  assert.equal(next.players[seat].resources.gold ?? 0, 0);
  assert.ok(!researchDecision(view(state, 1 - seat)));
  assert.ok(view(state, 1 - seat).advances.every((a) => !a.action));
  assert.ok(!researchDecision(JSON.parse(engine.webView(engine.stripSecret(state)))));
});

test('map position choices select, replace and deselect without submitting until confirmed', () => {
  const raw = JSON.parse(fixture('incidents/barbarians_move'));
  const barbarians = raw.players.find((p: { civilization: string }) => p.civilization === 'Barbarians');
  barbarians.units = [];
  barbarians.cities = [];
  const state = engine.tryMove(
    JSON.stringify(raw),
    JSON.stringify({ Response: { SelectAdvance: 'Storage' } }),
    0,
  );
  const v = view(state);
  const d = v.decision!;
  const options = mapDecisionOptions(d);
  assert.equal(options.length, 2);
  let selected = toggleDecisionSelection(d, [], 0);
  selected = toggleDecisionSelection(d, selected, 1);
  assert.deepEqual(selected, [1]);
  assert.deepEqual(toggleDecisionSelection(d, selected, 1), []);
  assert.deepEqual(toggleDecisionSelection(d, selected, 99), selected);
  const action = JSON.parse(
    engine.webQuery(
      engine.stripSecret(state, 0),
      0,
      JSON.stringify({
        kind: 'decision',
        values: selected.map((i) => options[i].value),
        payments: [],
      }),
    ),
  ).action;
  const next = JSON.parse(engine.tryMove(state, JSON.stringify(action), 0));
  assert.ok(
    next.players
      .find((p: { civilization: string }) => p.civilization === 'Barbarians')
      .cities.some((c: { position: string }) => c.position === options[1].position),
  );
  assert.equal(mapDecisionOptions(view(state, 1).decision).length, 0);
  const multiple = { ...d, max: 2, options: [...options, { ...options[0], value: 'C4', position: 'C4' }] };
  assert.deepEqual(toggleDecisionSelection(multiple, [0, 1], 2), [0, 1]);
  assert.equal(
    mapDecisionOptions({ ...d, options: options.map((o, i) => ({ ...o, value: i })) }).length,
    0,
    'Unit choices must not be confused with tile choices',
  );
});

test('payment choices include optional purchase/decline and complete mixed reward allocations', () => {
  const raw = JSON.parse(fixture('incidents/barbarians_move'));
  raw.incidents_left = [22];
  const barbarians = raw.players.find((p: { civilization: string }) => p.civilization === 'Barbarians');
  barbarians.units = [];
  barbarians.cities = [];
  let state = engine.tryMove(
    JSON.stringify(raw),
    JSON.stringify({ Response: { SelectAdvance: 'Storage' } }),
    0,
  );
  for (let i = 0; i < 8; i++) {
    const d = view(state).decision!;
    if (d.fields.length) break;
    const seat = engine.currentPlayer(state);
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(state, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: d.options.slice(0, d.min).map((o) => o.value),
          payments: [],
        }),
      ),
    ).action;
    state = engine.tryMove(state, JSON.stringify(action), seat);
  }
  const d = view(state).decision!;
  assert.equal(d.name, 'Great Scientist');
  assert.match(d.description, /When played \(1 action\).*Science/);
  assert.deepEqual(d.fields[0].choices, [{ culture_tokens: 1 }, {}]);
  const before = JSON.parse(state);
  for (const payment of d.fields[0].choices!) {
    const seat = engine.currentPlayer(state);
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(state, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: [],
          payments: [payment],
        }),
      ),
    ).action;
    const next = JSON.parse(engine.tryMove(state, JSON.stringify(action), seat));
    if (payment.culture_tokens) {
      assert.ok(next.players[seat].action_cards.includes(122));
      assert.equal(
        next.players[seat].resources.culture_tokens,
        before.players[seat].resources.culture_tokens - 1,
      );
    } else {
      assert.equal(
        next.players[seat].resources.culture_tokens,
        before.players[seat].resources.culture_tokens,
      );
      assert.deepEqual(view(JSON.stringify(next)).decision!.fields[0].choices, [{ culture_tokens: 2 }, {}]);
    }
  }
  const reward = fixture('advances/trade_routes_with_currency.outcome');
  const rewardView = view(reward);
  const choices = rewardView.decision!.fields[0].choices!;
  assert.ok(choices.some((p) => p.food === 1 && p.gold === 1));
  assert.ok(choices.some((p) => p.gold === 2));
  const seat = engine.currentPlayer(reward);
  for (const payment of choices) {
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(reward, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: [],
          payments: [payment],
        }),
      ),
    ).action;
    assert.doesNotThrow(() => engine.tryMove(reward, JSON.stringify(action), seat));
  }
});

test('Black Death selects the exact unit on a crowded tile and only removes it on confirmation', () => {
  const state = fixture('incidents/pandemics/black_death.outcome');
  const d = view(state, 0).decision!;
  const options = mapDecisionOptions(d);
  assert.equal(options.length, 7);
  assert.equal(new Set(options.map((o) => o.position)).size, 1);
  const elephant = options.findIndex(
    (o) => o.mapTarget?.kind === 'unit' && o.mapTarget.unitType === 'Elephant',
  );
  const target = options[elephant].mapTarget!;
  assert.equal(target.kind, 'unit');
  if (target.kind !== 'unit') throw new Error('Expected unit target');
  assert.equal(
    mapDecisionIndex(d, 'C2', { kind: 'unit', player: target.player, unit: target.unit }),
    elephant,
  );
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'unit', player: 1, unit: target.unit }), -1);
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'city', player: 0 }), -1);
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'units', player: 0 }), -1);
  assert.equal(mapDecisionIndex(d, 'B2', { kind: 'decision', decisionIndex: elephant }), -1);
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'decision', decisionIndex: 99 }), -1);
  assert.deepEqual(toggleDecisionSelection(d, [0], elephant), [elephant]);
  const query = (values: unknown[]) =>
    JSON.parse(
      engine.webQuery(
        engine.stripSecret(state, 0),
        0,
        JSON.stringify({ kind: 'decision', values, payments: [] }),
      ),
    );
  assert.throws(() => query([]));
  assert.throws(() => query([options[0].value, options[1].value]));
  const { action } = query([options[elephant].value]);
  const next = JSON.parse(engine.tryMove(state, JSON.stringify(action), 0));
  assert.deepEqual(
    next.players[0].units.map((u: { id: number }) => u.id),
    JSON.parse(state)
      .players[0].units.filter((u: { id: number }) => u.id !== target.unit)
      .map((u: { id: number }) => u.id),
  );
  assert.equal(mapDecisionOptions(view(state, 1).decision).length, 0);
  assert.equal(mapDecisionOptions(JSON.parse(engine.webView(engine.stripSecret(state))).decision).length, 0);
});

test('Earthquake offers separate building choices at the same city and respects the selection limit', () => {
  const raw = fixture('incidents/earthquake/earthquake');
  const state = engine.tryMove(
    raw,
    JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
    0,
  );
  const d = view(state, 0).decision!;
  const options = mapDecisionOptions(d);
  assert.ok(options.length > 3);
  assert.ok(options.every((o) => o.mapTarget?.kind === 'structure'));
  const sameCity = options.flatMap((o, i) => (o.position === 'C2' ? [i] : []));
  assert.ok(sameCity.length >= 2);
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'city', player: 0 }), -1);
  assert.equal(mapDecisionIndex(d, 'C2', { kind: 'decision', decisionIndex: sameCity[1] }), sameCity[1]);
  let selected: number[] = [];
  for (let i = 0; i < options.length; i++) selected = toggleDecisionSelection(d, selected, i);
  assert.equal(selected.length, d.max);
});
