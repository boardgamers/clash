import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (g: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), g.current_player_index));
const view = (g: any) =>
  JSON.parse(
    engine.webView(engine.stripSecret(JSON.stringify(g), g.current_player_index), g.current_player_index),
  );
const destinations = (g: any, city?: string, units: number[] = []) =>
  JSON.parse(
    engine.webQuery(
      engine.stripSecret(JSON.stringify(g), g.current_player_index),
      g.current_player_index,
      JSON.stringify({ kind: 'movement', city, units }),
    ),
  ).destinations;

async function setup() {
  let g = JSON.parse(
    await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'nomads-exploration', {}),
  );
  for (const civilization of ['Huns', 'Rome']) {
    g = JSON.parse(
      engine.tryMove(
        JSON.stringify(g),
        JSON.stringify({ ChooseCivilization: civilization }),
        engine.currentPlayer(JSON.stringify(g)),
      ),
    );
  }
  const p = g.players[g.current_player_index];
  p.units = [];
  p.advances.push('Storage');
  p.cities = ['C2', 'D1', 'D2'].map((position) => ({ position, mood_state: 'Neutral' }));
  g.actions_left = 1;
  return g;
}

test('Nomads exploration on the last action keeps the other two city moves', async () => {
  const g = await setup(),
    seat = g.current_player_index;
  const action = destinations(g, 'C2').find((d: any) => d.position === 'C3').action;
  let after = move(g, action);
  assert.ok(view(after).explorationDecision, 'fixture requires choosing the revealed block rotation');
  assert.deepEqual(view(after).nomadCities, [], 'finish the exploration choice before moving another city');
  assert.equal(after.state.Movement?.movement_actions_left, 2, 'pending exploration must retain movement');
  assert.equal(after.actions_left, 0);
  after = move(after, view(after).explorationDecision.choices[0].action);
  assert.equal(after.current_player_index, seat);
  assert.equal(after.state.Movement.movement_actions_left, 2);
  assert.equal(destinations(after, 'C3').length, 0, 'the same city cannot move twice');
  for (const [i, city] of ['D1', 'D2'].entries()) {
    const next = destinations(after, city).find((d: any) => d.terrain !== 'Unexplored');
    assert.ok(next, 'another city can move without spending a second action');
    after = move(after, next.action);
    assert.equal(after.actions_left, 0);
    assert.equal(after.current_player_index, seat);
    if (!i) assert.equal(after.state.Movement.movement_actions_left, 1);
  }
  assert.equal(after.state, 'Playing', 'three moves exhaust the Move action');
});

test('unmoved units can continue after a Nomads exploration choice', async () => {
  const g = await setup();
  g.players[g.current_player_index].units = [{ id: 10, unit_type: 'Settler', position: 'D2' }];
  g.players[g.current_player_index].next_unit_id = 11;
  let after = move(g, destinations(g, 'C2').find((d: any) => d.position === 'C3').action);
  after = move(after, view(after).explorationDecision.choices[0].action);
  const next = destinations(after, undefined, [10]).find((d: any) => d.terrain !== 'Unexplored');
  assert.ok(next);
  after = move(after, next.action);
  assert.equal(after.actions_left, 0);
  assert.equal(after.state.Movement.movement_actions_left, 1);
});

test('exploration on the third move does not grant extra movement', async () => {
  const g = await setup();
  g.actions_left = 0;
  g.state = { Movement: { movement_actions_left: 1 } };
  let after = move(g, destinations(g, 'C2').find((d: any) => d.position === 'C3').action);
  assert.equal(after.state, 'Playing');
  after = move(after, view(after).explorationDecision.choices[0].action);
  assert.equal(after.state, 'Playing');
  assert.equal(after.actions_left, 0);
  assert.equal(destinations(after, 'D1').length, 0);
});

test('the city still at its origin during exploration is not counted as another movable city', async () => {
  const g = await setup();
  g.players[g.current_player_index].cities = [g.players[g.current_player_index].cities[0]];
  let after = move(g, destinations(g, 'C2').find((d: any) => d.position === 'C3').action);
  assert.ok(view(after).explorationDecision);
  assert.equal(after.state, 'Playing', 'no other city or unit can use the remaining moves');
  after = move(after, view(after).explorationDecision.choices[0].action);
  assert.equal(after.state, 'Playing');
  assert.equal(after.actions_left, 0);
});
