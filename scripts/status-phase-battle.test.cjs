// Build the Node WASM package first; optionally point CLASH_WASM at a local build.
// Run: CLASH_WASM=/tmp/clash-status-wasm/server.js node --test scripts/status-phase-battle.test.cjs
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const engine = require(process.env.CLASH_WASM || '../server/pkg/server.js');

function fixture() {
  const game = JSON.parse(readFileSync(resolve(__dirname,
    '../server/tests/test_games/incidents/barbarians_attack.json')));
  game.incidents_left = [3]; // Famine, including barbarian movement.
  game.players[0].units.forEach(unit => { unit.unit_type = 'Infantry'; });
  game.dice_roll_outcomes = Array(40).fill(10);
  return game;
}

// Each call crosses the actual serialized production WASM API boundary.
function move(game, action, player) {
  return JSON.parse(engine.move(JSON.stringify(game), JSON.stringify(action), player));
}

const hasBattle = unit => (unit.movement_restrictions || []).includes('Battle');
const infantryMove = { Movement: { Move: { units: [7], destination: 'B1', payment: {} } } };

test('status-phase Famine defense allows movement next age', () => {
  let game = move(fixture(), { Response: { SelectAdvance: 'Storage' } }, 0);
  assert.equal(game.players[0].units.length, 1);
  assert.equal(hasBattle(game.players[0].units[0]), false);
  game = move(game, { Response: { SelectAdvance: 'Storage' } }, 1);
  game = move(game, { Response: { SelectPositions: [] } }, 0);
  game = move(game, { Response: { SelectPositions: [] } }, 1);
  game = move(game, { Response: { SelectPlayer: 0 } }, 0);
  assert.equal(game.age, 2);
  game = move(game, infantryMove, 0);
  assert.equal(game.players[0].units[0].position, 'B1');
});

test('paid-advance Famine defense still stops the fighters for this turn', () => {
  let game = fixture();
  game.events = [];
  game = move(game, { Playing: { Advance: { advance: 'Storage', payment: { ideas: 2 } } } }, 0);
  assert.equal(hasBattle(game.players[0].units[0]), true);
  assert.throws(
    () => engine.tryMove(JSON.stringify(game), JSON.stringify(infantryMove), 0),
    error => /battle movement restriction/.test(String(error)),
  );
});
