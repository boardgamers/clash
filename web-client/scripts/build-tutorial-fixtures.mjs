import { createRequire } from 'node:module';
import fs from 'node:fs/promises';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const directory = new URL('../src/tutorial/positions/', import.meta.url);
const base = JSON.parse(await engine.init(2, [], {}, 'clash-tutorial-v1', {}));
for (const [position, terrain] of Object.entries({
  D2: 'Fertile',
  D7: 'Fertile',
  C2: 'Forest',
  E2: 'Mountain',
  D1: 'Water',
}))
  base.map.tiles.find((t) => t[0] === position)[1] = terrain;
base.current_player_index = 0;
base.starting_player_index = 0;
base.players[0].name = 'You';
base.players[1].name = 'Opponent';
base.players[0].civilization = 'Rome';
base.players[0].cities = [{ position: 'D2', mood_state: 'Happy' }];
base.players[0].units = [{ position: 'D2', unit_type: 'Settler', id: 0 }];
base.players[0].available_leaders = ['Caesar', 'Augustus', 'Sulla'];
base.players[0].resources = {
  food: 2,
  wood: 3,
  ore: 3,
  ideas: 3,
  gold: 3,
  mood_tokens: 7,
  culture_tokens: 7,
};
base.players[0].advances = ['Farming', 'Mining'];
base.players[1].cities = [{ position: 'D7', mood_state: 'Happy' }];
base.players[1].units = [{ position: 'D7', unit_type: 'Settler', id: 0 }];
async function fixture(name) {
  const game = JSON.parse(
    await fs.readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of base.players.slice(2))
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length, cities: [], units: [] });
  game.current_player_index = 0;
  return game;
}
for (const id of [
  'first-turn',
  'city-economy',
  'activation-happiness',
  'research-paths',
  'movement-founding',
  'sea-transport',
  'cultural-influence',
  'incidents-hostiles',
  'cards-leaders',
  'wonders-ownership',
  'objectives-ages',
  'combat-barbarians',
  'pirates',
  'action-cards',
]) {
  let g = structuredClone(base),
    p = g.players[0];
  if (id === 'city-economy') {
    p.cities[0].city_pieces = { temple: 0 };
    p.advances.push('Myths');
    p.cities.push({ position: 'E3', mood_state: 'Happy' });
    g.map.tiles.find((t) => t[0] === 'E3')[1] = 'Fertile';
  }
  if (id === 'activation-happiness') {
    p.cities[0].mood_state = 'Neutral';
    p.cities[0].activations = 1;
    p.cities.push({ position: 'E3', mood_state: 'Angry' });
    g.map.tiles.find((t) => t[0] === 'E3')[1] = 'Fertile';
  }
  if (id === 'research-paths') p.advances.push('Writing');
  if (id === 'movement-founding') {
    p.units.push({ id: 1, position: 'D2', unit_type: 'Settler' });
    p.next_unit_id = 2;
  }
  if (id === 'sea-transport') {
    // Reveal the entire sea-route block. Partly revealed blocks would overwrite
    // occupied water when an adjacent tile is explored later.
    g.map.unexplored_blocks = g.map.unexplored_blocks.filter((block) => block.position.top_tile !== 'B2');
    for (const [position, terrain] of Object.entries({
      B2: 'Fertile',
      A3: 'Barren',
      C3: 'Water',
      B3: 'Water',
    }))
      g.map.tiles.find((tile) => tile[0] === position)[1] = terrain;
    g.map.tiles.find((t) => t[0] === 'C2')[1] = 'Water';
    g.map.tiles.find((t) => t[0] === 'D1')[1] = 'Water';
    g.map.unexplored_blocks.find((block) => block.position.top_tile === 'B4').block.terrain = [
      'Forest',
      'Mountain',
      'Fertile',
      'Mountain',
    ];
    p.advances.push('Fishing', 'Tactics');
    p.units.push({ id: 1, position: 'D1', unit_type: 'Ship' });
    p.next_unit_id = 2;
  }
  if (id === 'cultural-influence') {
    g = await fixture('base/cultural_influence_instant');
    g.dice_roll_outcomes = [0];
    g.players[0].civilization = 'Rome';
    g.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
    g.players[0].resources.culture_tokens = 7;
    g.players[1].cities[0].city_pieces = { temple: 1 };
  }
  if (id === 'incidents-hostiles') {
    g = await fixture('incidents/earthquake/flood');
    g.players[0].advances.push('Myths');
  }
  if (id === 'cards-leaders') {
    p.advances.push('Tactics', 'Writing', 'Math');
    p.available_leaders = ['Caesar', 'Augustus', 'Sulla'];
  }
  if (id === 'wonders-ownership') {
    g.map.tiles.find((t) => t[0] === 'D1')[1] = 'Water';
    p.advances.push('Fishing', 'Myths');
    p.cities[0].city_pieces = { wonders: ['GreatLighthouse'] };
    p.resources.wood = 2;
  }
  if (id === 'objectives-ages') {
    g = await fixture('objective_cards/instant/draft');
  }
  if (id === 'combat-barbarians') {
    p.advances.push('Tactics');
    p.action_cards = [20];
    p.units = [0, 1, 2].map((id) => ({id, position: 'D2', unit_type: 'Infantry'}));
    p.next_unit_id = 3;
    const barbarians = g.players.find((p) => p.civilization === 'Barbarians');
    barbarians.cities = [{position: 'C2', mood_state: 'Neutral'}];
    barbarians.units = [{id: 0, position: 'C2', unit_type: 'Infantry'}];
    barbarians.next_unit_id = 1;
    // A fixed teaching roll: Peltasts succeeds, the army wins without casualties.
    g.dice_roll_outcomes = [11, 9, 0, 0, 11];
  }
  if (id === 'pirates') {
    p.advances.push('Fishing', 'Tactics');
    p.action_cards = [];
    p.resources.food = 0;
    p.units = [0, 1].map((id) => ({id, position: 'D1', unit_type: 'Ship'}));
    p.next_unit_id = 2;
    g.map.tiles.find(([position]) => position === 'C2')[1] = 'Water';
    const pirates = g.players.find((p) => p.civilization === 'Pirates');
    pirates.units = [{id: 0, position: 'C2', unit_type: 'Ship'}];
    pirates.next_unit_id = 1;
    g.dice_roll_outcomes = [0, 11, 11];
  }
  if (id === 'action-cards') {
    p.action_cards = [20];
    p.cities[0].city_pieces = {academy: 0};
    p.resources.wood = 0;
  }
  // Older test fixtures serialize token fields, but only goods have storage limits.
  for (const player of g.players) {
    for (const token of ['mood_tokens', 'culture_tokens', 'captives']) delete player.resource_limit?.[token];
  }
  // Validate the fixture against the production engine before saving it.
  engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0);
  await fs.writeFile(new URL(`${id}.json`, directory), JSON.stringify(g, null, 2) + '\n');
}
console.log('Wrote validated tutorial positions');
