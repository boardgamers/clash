// Run after build:bridge: node scripts/timing-check.mjs
// Test real WASM exports: the protocol needs a JS array, not a JSON string,
// Uint32Array or bigint[]. Both move and tryMove must obey the same policy.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile} from 'node:fs/promises';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = name => readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8');
const move = (raw, player, action, method = 'move') => engine[method](raw, JSON.stringify(action), player);
function credits(raw, expected) {
  const actual = engine.timeIncrements(raw);
  assert(Array.isArray(actual), 'timeIncrements must return an actual JS array');
  assert(actual.every(value => Number.isSafeInteger(value) && value >= 0));
  assert.deepEqual(actual, expected);
}
assert.equal(typeof engine.timeIncrements, 'function');

for (const method of ['move', 'tryMove']) {
  let raw = await fixture('advances/collect_free_economy');
  credits(raw, [0, 0]);
  const data = JSON.parse(raw);
  data.time_increments = [11, 5];
  raw = JSON.stringify(data);
  const collect = {Playing: {Collect: {city_position: 'C2', collections: [{position: 'B1', pile: {ore: 1}, times: 1}], action_type: 'Collect'}}};
  for (let n = 0; n < 20; n++) {
    raw = move(raw, 0, collect, method);
    credits(raw, [11, 5]);
    raw = move(raw, 0, 'Undo', method);
    credits(raw, [11, 5]);
    raw = move(raw, 0, 'Redo', method);
    credits(raw, [11, 5]);
    raw = move(raw, 0, 'Undo', method);
    credits(raw, [11, 5]);
  }
  raw = move(raw, 0, {Playing: 'EndTurn'}, method);
  credits(raw, [12, 5]);

  raw = await fixture('action_cards/spy');
  const rng = JSON.parse(raw).rng;
  raw = move(raw, 0, {Playing: {ActionCard: 7}}, method);
  credits(raw, [0, 0]);
  raw = move(raw, 0, {Response: {Payment: [{culture_tokens: 1}]}}, method);
  credits(raw, [1, 0]);
  assert.equal(JSON.parse(raw).rng, rng, 'Spy reveals information without RNG changes');
  const declined = move(raw, 0, {Response: {SelectHandCards: []}}, method);
  credits(declined, [1, 0]);
  raw = move(raw, 0, {Response: {SelectHandCards: [{ActionCard: 1}, {ActionCard: 2}]}}, method);
  credits(raw, [2, 0]);

  raw = await fixture('movement/explore_resolution');
  raw = move(raw, 1, {Movement: {Move: {units: [0], destination: 'D6'}}}, method);
  credits(raw, [0, 1]);
  raw = move(raw, 1, {Response: {ExploreResolution: 3}}, method);
  credits(raw, [0, 1]);
  raw = move(raw, 1, 'Undo', method);
  credits(raw, [0, 1]);

  raw = await fixture('base/same_player_undo');
  raw = move(raw, 0, {Playing: {Construct: {city_position: 'C2', city_piece: 'Observatory', payment: {food: 1, wood: 1, ore: 1}}}}, method);
  credits(raw, [1, 0]);

  const trade = JSON.parse(await fixture('action_cards/tech_trade'));
  trade.round = 2;
  trade.players[0].advances.push('Irrigation');
  raw = move(JSON.stringify(trade), 0, {Playing: {ActionCard: 39}}, method);
  credits(raw, [1, 0]);
  const {key} = JSON.parse(engine.webPlanning(raw, 0));
  for (const waiting of [true, false, true, false]) {
    raw = move(raw, 0, {QueueMoves: {key, actions: [{Playing: 'EndTurn'}], waiting}}, method);
    credits(raw, [1, 0]);
  }
  raw = move(raw, 1, {Response: {SelectAdvance: 'Fishing'}}, method);
  credits(raw, [2, 1]); // Responder's handoff + queued owner's EndTurn.

  raw = await engine.init(2, [], {civilization: 'DraftThree'}, 'timing-js-draft', {});
  const offers = JSON.parse(raw).civilization_draft.offers;
  credits(raw, [0, 0]);
  for (const choice of offers[0]) {
    raw = move(raw, 0, {ChooseCivilization: choice}, method);
    credits(raw, [1, 0]);
  }
  raw = move(raw, 1, {ChooseCivilization: offers[1][0]}, method);
  credits(raw, [1, 1]);
  for (let n = 0; n < 5; n++) {
    raw = engine.setPlayerSettings(raw, 0, {skipRazeCity: n % 2 === 0, keepGovernment: n % 2 !== 0});
    credits(raw, [1, 1]);
  }
  credits(engine.replay(raw, {}), [1, 1]);
  for (const viewer of [0, 1, undefined]) credits(engine.stripSecret(raw, viewer), [1, 1]);
}
console.log('Clock increments: JS array contract, move/tryMove, undo/redo, reveals, handoffs, queues, draft and settings passed');
