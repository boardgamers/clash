// City-scoped Tributes legality through ordinary and private queue WASM APIs.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const { battle, objectives, engine, move, planning } = require('./postcombat-queue-fixture.cjs');
function fixture(queued, large = false) {
  const game = JSON.parse(readFileSync(resolve(__dirname, '../server/tests/test_games/advances/collect_free_economy.json')));
  game.players.push({ id: 2, civilization: 'Barbarians', next_unit_id: 0 },
    { id: 3, civilization: 'Pirates', next_unit_id: 0 });
  const p = game.players[0];
  p.civilization = 'Aztecs';
  p.advances = ['Farming', 'Mining'];
  p.resources = { food: 1, wood: 1, mood_tokens: 6 };
  p.resource_limit.food = 7;
  p.units = [{ id: 0, position: 'A1', unit_type: { Leader: 'Acamapichtli' } }];
  p.next_unit_id = 1;
  p.recruited_leaders = ['Acamapichtli'];
  if (large) p.cities[0].city_pieces = { temple: 0, academy: 0, fortress: 0 };
  if (queued) game.events = objectives(battle()).events;
  return game;
}
const collect = city => ({ Playing: { Collect: {
  city_position: city,
  collections: [{ position: city === 'A1' ? 'A2' : 'B2', pile: { food: 1 }, times: 1 }],
  action_type: { Special: { Modifier: 'Tributes' } },
} } });
for (const queued of [false, true]) test(`Tributes keeps city eligibility ${queued ? 'in queue projection' : 'in ordinary execution'}`, () => {
  const game = fixture(queued);
  function play(state, action) {
    if (!queued) return move(state, action, 0);
    const info = planning(state);
    assert.ok(info);
    return move(state, { QueueMoves: { key: info.key, actions: [action], waiting: false } }, 0);
  }
  // A non-leader city must not acquire the modifier just because A1 qualifies.
  assert.throws(() => play(game, collect('C2')), error => /Tributes requires Acamapichtli/.test(String(error)));
  assert.throws(() => play(fixture(queued, true), collect('A1')), error => /Tributes requires Acamapichtli/.test(String(error)));
  // Use the engine's actual choices instead of assuming a terrain resource.
  const view = JSON.parse(engine.webView(JSON.stringify(game), 0));
  // The queued view is projected by webPlanning, so build choices on the plain fixture.
  const ordinary = JSON.parse(engine.webView(JSON.stringify(fixture(false)), 0));
  const city = ordinary.cities.find(c => c.position === 'A1');
  const choice = city.choices[0];
  const action = collect('A1');
  action.Playing.Collect.collections = [{ position: choice.position, pile: choice.pile, times: 1 }];
  assert.ok(view);
  assert.doesNotThrow(() => play(game, action));
});
