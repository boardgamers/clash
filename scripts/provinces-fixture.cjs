// Synthetic public-engine Rome capture fixtures.
const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const move = (g, a) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(a), 0));
const capture = g => move(g, { Movement: { Move: { units: [0], destination: 'C1', payment: {} } } });
const city = g => g.players[0].cities.find(c => c.position === 'C1');
async function setup(culture = 0, mood = 0, arena = false, last = true) {
  const g = JSON.parse(await engine.init(2, [], {}, 'provinces-printed-rule', null));
  g.current_player_index = 0;
  g.starting_player_index = 0;
  g.map = { tiles: [['C2', 'Fertile'], ['C1', 'Fertile'], ['D1', 'Fertile']], unexplored_blocks: [] };
  g.players.forEach((p, i) => {
    p.civilization = ['Rome', 'China', 'Barbarians', 'Pirates'][i];
    p.objective_cards = []; p.action_cards = []; p.units = [];
    p.cities = i === 0 ? [{position: 'C2', mood_state: 'Happy'}] : i === 1 ? [{position: 'C1', mood_state: 'Happy'}, ...(!last ? [{position: 'D1', mood_state: 'Happy'}] : [])] : [];
    p.advances = ['Farming', 'Mining', 'Tactics', ...(i === 0 ? ['Nationalism'] : [])];
  });
  g.players[0].resources = {culture_tokens: culture, mood_tokens: mood};
  if (arena) g.players[0].cities[0].city_pieces = {wonders: ['Colosseum']};
  g.players[0].units = [{id: 0, position: 'C2', unit_type: 'Infantry'}];
  g.players[0].next_unit_id = 1;
  return g;
}
module.exports = {engine, move, capture, city, setup};
