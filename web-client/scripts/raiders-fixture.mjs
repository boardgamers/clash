import { readFileSync } from 'node:fs';

/** Synthetic public board built from a checked-in rules fixture, never a live game. */
export async function raidersFixture(engine) {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/advances/trade_routes.json', import.meta.url), 'utf8'));
  const npcs = JSON.parse(await engine.init(2, [], {}, 'raiders-guide', {})).players.slice(2);
  for (const npc of npcs) if (!game.players.some(p => p.civilization === npc.civilization)) game.players.push({ ...npc, id: game.players.length });
  game.current_player_index = 0; // Inspect Greece off-turn.
  game.players[0].civilization = 'Huns';
  game.players[0].advances.push('Tactics');
  game.players[0].special_advances = ['Raiders'];
  game.players[0].units = [{ position: 'D9', unit_type: 'Infantry', id: 18 }];
  game.players[0].cities = [
    { position: 'D9', mood_state: 'Happy' },
    { position: 'B7', mood_state: 'Happy' },
  ];
  game.players[0].next_unit_id = 19;
  game.players[1].units = [{ position: 'D7', unit_type: 'Ship', id: 8, carried_units: [
    { unit_type: 'Settler', id: 6 }, { unit_type: 'Settler', id: 7 },
  ] }];
  game.players[1].next_unit_id = 9;
  const tiles = new Map(game.map.tiles);
  for (const [position, terrain] of [['D7', 'Water'], ['D8', 'Fertile'], ['D9', 'Fertile'], ['B7', 'Fertile'], ['C7', 'Fertile']]) tiles.set(position, terrain);
  game.map.tiles = [...tiles];
  return game;
}
