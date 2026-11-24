// Public WASM API regressions. Build with bash web-client/scripts/build-bridge.sh.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const read = name => JSON.parse(fs.readFileSync(path.join(__dirname, '../server/tests/', name)));
const move = (g, action, player = 0) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), player));
const view = (g, player = 0) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), player), player));
const payment = { Response: { Payment: [{ food: 2, wood: 3, ore: 6, gold: 1, culture_tokens: 2 }] } };
function game() {
  const g = read('test_games/incidents/great_persons/great_architect.json');
  g.players[0].action_cards = [155];
  g.actions_left = 3;
  for (const civilization of ["Barbarians", "Pirates"]) {
    g.players.push({ id: g.players.length, civilization, resources: {}, cities: [], units: [], advances: [], next_unit_id: 0 });
  }
  return g;
}
function build(g) {
  g = move(g, { Playing: { ActionCard: 155 } });
  assert.equal(g.actions_left, 3);
  g = move(g, { Response: { SelectPositions: ['A1'] } });
  assert.equal(g.actions_left, 2);
  g = move(g, payment);
  assert.equal(g.actions_left, 2);
  assert.ok(g.players[0].wonders_built.includes('Pyramids'));
  return g;
}
test('normal and Japan public Architect cost precisely one construction action', () => {
  build(game());
  const japan = game();
  japan.players[0].civilization = 'Japan';
  japan.players[0].special_advances = ['Shogunate'];
  const g = build(japan);
  assert.ok(!g.players[0].event_info?.['Shogunate card']);
});
test('zero actions, missing Engineering, and no happy cities never expose playable Architect', () => {
  for (const change of [g => g.actions_left = 0, g => g.players[0].advances = ['Farming', 'Mining'], g => g.players[0].cities.forEach(c => c.mood_state = 'Angry')]) {
    const g = game(); change(g);
    assert.equal(view(g).actionCards.find(c => c.id === 155).action, null);
    assert.throws(() => move(g, { Playing: { ActionCard: 155 } }));
    assert.ok(g.players[0].action_cards.includes(155));
  }
});
test('frozen published-engine 0.4.170 full replay preserves free wonder and all three following actions', () => {
  const f = read('fixtures/legacy_architect_170.json');
  const full = JSON.parse(engine.replay(JSON.stringify(f.history), {}));
  assert.equal(full.actions_left, 0);
  assert.ok(full.players[1].wonders_built.includes('GreatWall'));
  const before = JSON.parse(engine.replay(JSON.stringify(f.history), { to: f.before }));
  const newPlay = move(before, { Playing: { ActionCard: 155 } }, 1);
  assert.equal(newPlay.actions_left, 2);
  const pending = JSON.parse(engine.replay(JSON.stringify(f.history), { to: f.pending }));
  const resumed = move(pending, { Response: { Payment: [{ food: 3, wood: 2, ore: 7, culture_tokens: 2 }] } }, 1);
  assert.equal(resumed.actions_left, 3);
});
test('Nebuchadnezzar and Qin keep free wonders only in their own city', () => {
  for (const [civilization, leader] of [['Babylonia', 'Nebuchadnezzar'], ['China', 'Qin']]) {
    const g = game();
    g.players[0].civilization = civilization;
    g.players[0].units[0] = { id: 0, position: 'A1', unit_type: { Leader: leader } };
    g.players[0].recruited_leaders = [leader];
    g.actions_left = 0;
    assert.ok(view(g).actionCards.find(c => c.id === 155).action);
    let next = move(g, { Playing: { ActionCard: 155 } });
    // Only the leader's city is eligible, so no city-choice prompt remains.
    assert.equal(next.events.at(-1).event_type.WonderCard.selected_position, 'A1');
    next = move(next, payment);
    assert.equal(next.actions_left, 0);
    assert.ok(next.players[0].wonders_built.includes('Pyramids'));
  }
});
test('Great Engineer still pays for the card, offsets construction and does not activate city', () => {
  let g = read('test_games/incidents/great_persons/great_engineer.json');
  g.players[0].action_cards = [126];
  g.actions_left = 3;
  const cityBefore = structuredClone(g.players[0].cities.find(c => c.position === 'C2'));
  g = move(g, { Playing: { ActionCard: 126 } });
  assert.equal(g.actions_left, 2);
  g = move(g, { Response: { SelectAdvance: 'Engineering' } });
  g = move(g, { Response: { Bool: true } });
  assert.equal(g.actions_left, 3);
  g = move(g, { Playing: { Construct: { city_position: 'C2', city_piece: 'Fortress', payment: { food: 1, wood: 1, ore: 1 } } } });
  assert.equal(g.actions_left, 2);
  const cityAfter = g.players[0].cities.find(c => c.position === 'C2');
  assert.equal(cityAfter.mood_state, cityBefore.mood_state);
  assert.equal(cityAfter.activations, cityBefore.activations);
  assert.equal(cityAfter.angry_activation, cityBefore.angry_activation);
});
test('Architect undo/redo retains old free play while replacing its future charges once', () => {
  const f = read('fixtures/legacy_architect_170.json');
  for (const legacy of [false, true]) {
    let pending = JSON.parse(engine.replay(JSON.stringify(f.history), { to: f[legacy ? 'pending' : 'before'] }));
    if (!legacy) pending = move(pending, { Playing: { ActionCard: 155 } }, 1);
    const boundary = pending.great_architect_action_cost_from;
    const undone = move(pending, 'Undo', 1);
    assert.equal(undone.actions_left, 3);
    assert.equal(undone.great_architect_action_cost_from, boundary);
    const redone = move(undone, 'Redo', 1);
    assert.equal(redone.actions_left, legacy ? 3 : 2);
    assert.equal(redone.great_architect_action_cost_from, boundary);
    const branch = move(undone, { Playing: { ActionCard: 155 } }, 1);
    assert.equal(branch.actions_left, 2);
    assert.equal(branch.great_architect_action_cost_from, f.pending);
  }
});
