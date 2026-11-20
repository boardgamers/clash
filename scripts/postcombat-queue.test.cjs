// Build web-client/.engine with scripts/build-bridge.sh before running.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { battle, objectives, engine, move, planning, stripped } = require('./postcombat-queue-fixture.cjs');
const stop = { Movement: 'Stop' };
const response = cards => ({ Response: { SelectHandCards: cards.map(ObjectiveCard => ({ ObjectiveCard })) } });

for (const result of ['win', 'draw', 'loss']) for (const claim of [false, true]) {
  test(`${result}: private queue survives objective ${claim ? 'claim' : 'skip'} and serialized handoff`, () => {
    let game = battle(result);
    const info = planning(game);
    assert.ok(info);
    game = move(game, { QueueMoves: { key: info.key, actions: [stop], waiting: true } }, 0);
    game = objectives(game);
    const next = planning(game);
    assert.equal(next.label, 'Choosing an objective');
    assert.equal(next.key, info.key);
    assert.equal(next.waiting, true);
    assert.deepEqual(next.actions, [stop]);
    assert.equal(game.state.Movement.movement_actions_left, 2);
    assert.equal(stripped(game, 1).action_plan, undefined);
    assert.equal(planning(game, 1), null);
    const owner = stripped(game, 0);
    assert.deepEqual(owner.events[0].handler.request.SelectHandCards.choices, []);
    assert.equal(owner.events[0].event_type.SelectObjectives.shown_objective, null);
    assert.throws(() => move(game, { QueueMoves: { key: 'stale', actions: [], waiting: false } }, 0));
    game = move(game, response(claim ? [27] : []), 1);
    assert.equal(game.state, 'Playing');
    assert.equal(game.action_plan, undefined);
    assert.equal((game.events || []).length, 0);
    assert.equal((game.players[1].completed_objectives || []).length, Number(claim));
  });
}

test('fresh objective queue has identical public and authoritative key; no opponent reveal', () => {
  let game = objectives(battle());
  const info = planning(game);
  assert.ok(info);
  assert.equal(info.key, JSON.parse(engine.webPlanning(JSON.stringify(game), 0)).key);
  const before = JSON.stringify(game);
  const projected = JSON.parse(info.projected);
  assert.equal(projected.state.Movement.movement_actions_left, 2);
  assert.equal(JSON.stringify(game), before);
  assert.equal(projected.players[1].objective_cards.every(card => card === 0), true);
  game = move(game, { QueueMoves: { key: info.key, actions: [], waiting: true } }, 0);
  game = move(game, response([]), 1);
  assert.equal(game.state.Movement.movement_actions_left, 2);
  assert.equal(game.action_plan, undefined);
});

test('all repeated objective opportunities finish before queue execution', () => {
  let game = objectives(battle('draw', true));
  const info = planning(game);
  game = move(game, { QueueMoves: { key: info.key, actions: [stop], waiting: true } }, 0);
  let prompts = 0;
  while ((game.events || []).length) {
    assert.equal(game.state.Movement.movement_actions_left, 2);
    assert.equal(planning(game).key, info.key);
    assert.equal(planning(game).waiting, true);
    game = move(game, response([]), 1);
    assert.ok(++prompts < 10);
  }
  assert.ok(prompts >= 3);
  assert.equal(game.state, 'Playing');
  assert.equal(game.action_plan, undefined);
});



test('replay revalidates and keeps only the failed private suffix', () => {
  let game = objectives(battle());
  const info = planning(game);
  const collect = { Playing: { Collect: { city_position: 'A1', collections: [{ position: 'A1', pile: { food: 1 }, times: 1 }], action_type: 'Collect' } } };
  const end = { Playing: 'EndTurn' };
  game = move(game, { QueueMoves: { key: info.key, actions: [stop, collect, end], waiting: false } }, 0);
  // Authoritative changes are revalidated, never trusted from a projection.
  game.actions_left = 0;
  game = move(game, response([]), 1);
  assert.equal(game.state, 'Playing');
  assert.equal(game.current_player_index, 0);
  assert.deepEqual(game.action_plan.actions, [collect, end]);
  assert.ok(game.action_plan.error);
  assert.equal(stripped(game, 1).action_plan, undefined);
});

test('legacy/unproven, status-phase, mandatory and nested objective prompts fail closed', () => {
  const game = objectives(battle());
  for (const change of [
    g => { delete g.events[0].event_type.SelectObjectives.combat_turn_owner; },
    g => { g.round = 4; },
    g => { g.events[0].handler.request.SelectHandCards.needed.start = 1; },
    g => { g.events.push(structuredClone(g.events[0])); },
  ]) {
    const changed = structuredClone(game);
    change(changed);
    assert.equal(planning(changed), null);
  }
});

test('own postcombat objectives require an actual choice, never an inferred claim', () => {
  let game = battle('draw', true, g => { g.players[0].objective_cards = [6]; });
  assert.equal(game.events[0].player, 0);
  assert.equal(planning(game), null);
  assert.equal((game.players[0].completed_objectives || []).length, 0);
  game = move(game, response([6]), 0);
  assert.equal(game.players[0].completed_objectives[0].card, 6);
  assert.ok(planning(game));
  assert.equal((game.players[1].completed_objectives || []).length, 0);
});

test('unfinished combat tactics do not offer a continuation queue', () => {
  const game = battle('win', false, g => { g.players[1].action_cards = [7]; });
  assert.equal(planning(game), null);
  assert.ok(game.events.some(e => e.event_type.CombatRoundStart));
});
