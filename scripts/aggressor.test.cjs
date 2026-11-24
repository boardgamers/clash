// Synthetic public-engine setup, real movement/combat, and serialized WASM claims.
// Build with: bash web-client/scripts/build-bridge.sh
const assert = require('node:assert/strict');
const { test } = require('node:test');
const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const move = (game, action, player = 0) => JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), player));

async function battle(card, attackers, defenders, owner = 0, barbarian = false) {
  const game = JSON.parse(await engine.init(2, [], {}, 'aggressor-printed-rule', null));
  game.current_player_index = 0;
  game.starting_player_index = 0;
  game.map = { tiles: [['C2', 'Fertile'], ['C1', 'Fertile']], unexplored_blocks: [] };
  game.players.forEach((p, i) => {
    p.civilization = ['Rome', 'Greece', 'Barbarians', 'Pirates'][i];
    p.objective_cards = i === owner ? [card] : [];
    p.action_cards = [];
    p.cities = [];
    p.units = [];
    p.advances = ['Farming', 'Mining', 'Tactics'];
  });
  const opponent = barbarian ? 2 : 1;
  for (const [index, count, position] of [[0, attackers, 'C2'], [opponent, defenders, 'C1']]) {
    game.players[index].units = Array.from({ length: count }, (_, id) => ({ id, position, unit_type: 'Infantry' }));
    game.players[index].next_unit_id = count;
  }
  game.dice_roll_outcomes = Array(10).fill(11);
  return move(game, { Movement: { Move: { units: Array.from({ length: attackers }, (_, i) => i), destination: 'C1', payment: {} } } });
}
const offered = g => (g.events || []).some(e => e.event_type.SelectObjectives?.shown_objective === 'Aggressor');

for (const card of [23, 25]) {
  for (const [attackers, defenders, result] of [[3, 2, 'win'], [2, 3, 'loss'], [2, 2, 'draw']]) {
    test(`O${card} attacker ${result}: two army casualties offer and claim through WASM`, async () => {
      let game = await battle(card, attackers, defenders);
      assert.ok(offered(game));
      game = move(game, { Response: { SelectHandCards: [{ ObjectiveCard: card }] } });
      assert.deepEqual(game.players[0].completed_objectives, [{ card, name: 'Aggressor' }]);
      assert.ok(!(game.players[0].objective_cards || []).includes(card));
      // stripSecret deserializes the completed save without removing its award.
      const reloaded = JSON.parse(engine.stripSecret(JSON.stringify(game), 0));
      assert.deepEqual(reloaded.players[0].completed_objectives, game.players[0].completed_objectives);
    });
  }
  test(`O${card} defender win, barbarian opponent, and one casualty never offer`, async () => {
    assert.ok(!offered(await battle(card, 2, 3, 1)));
    assert.ok(!offered(await battle(card, 3, 2, 0, true)));
    assert.ok(!offered(await battle(card, 2, 1)));
  });
}
