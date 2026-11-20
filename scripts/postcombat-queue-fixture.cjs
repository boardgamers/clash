const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const move = (game, action, player) => JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), player));
const stripped = (game, player) => JSON.parse(engine.stripSecret(JSON.stringify(game), player));
const planning = (game, player = 0) => JSON.parse(engine.webPlanning(JSON.stringify(stripped(game, player)), player));

function battle(result = 'win', extraObjectives = false, configure = () => {}) {
  const game = JSON.parse(readFileSync(resolve(__dirname, '../server/tests/test_games/tactics_cards/heavy_resistance.json')));
  for (const p of game.players) {
    p.objective_cards = [];
    p.action_cards = [];
    p.advances = ['Farming', 'Mining', 'Tactics'];
  }
  game.players[1].objective_cards = extraObjectives ? [27, 28, 5, 6] : [27, 28];
  const count = result === 'win' || extraObjectives ? 3 : 2;
  game.players[0].units = Array.from({ length: count }, (_, id) => ({ id, position: 'C2', unit_type: 'Infantry' }));
  game.players[0].units.push({ id: 30, position: 'C2', unit_type: 'Settler' });
  game.players[0].next_unit_id = 31;
  if (result === 'loss' || extraObjectives) game.players[1].units.push({ id: 20, position: 'C1', unit_type: 'Infantry' });
  for (const position of ['B1', 'B3']) game.players[1].cities.push({ position, mood_state: 'Neutral' });
  game.dice_roll_outcomes = Array(10).fill(11);
  configure(game);
  return move(game, { Movement: { Move: { units: Array.from({ length: count }, (_, id) => id), destination: 'C1', payment: {} } } }, 0);
}
function objectives(game) {
  if (game.events[0]?.event_type.CombatEnd) return move(game, { Response: { SelectPositions: ['B1'] } }, 1);
  return game;
}

module.exports = { battle, objectives, engine, move, planning, stripped };
