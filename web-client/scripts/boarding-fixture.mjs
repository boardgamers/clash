import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
export const engine = createRequire(import.meta.url)('../.engine/server.js');
const base = JSON.parse(await readFile(new URL('../../server/tests/test_games/movement/movement.json', import.meta.url), 'utf8'));
const fullMap = JSON.parse(await readFile(new URL('../../server/tests/test_games/movement/ship_navigation_unit_test.json', import.meta.url), 'utf8')).map;
const npcs = JSON.parse(await engine.init(2, [], {}, 'boarding', {})).players.slice(2);
export function fixture(pirates, count) {
  const state = structuredClone(base);
  state.state = 'Playing'; state.actions_left = 3;
  state.players.push(...structuredClone(npcs));
  for (const p of state.players) { p.units = []; p.cities = []; }
  const p = state.players[0];
  p.civilization = pirates ? 'Carthage' : 'Rome';
  p.advances = ['Farming','Mining','Tactics','Navigation'];
  p.next_unit_id = 22;
  p.units = Array.from({length:count}, (_,id) => ({id,position:'F7',unit_type:id < 2 ? 'Infantry' : 'Elephant'}));
  const owner = pirates ? state.players.find(p => p.civilization === 'Pirates') : p;
  owner.units.push(...(pirates ? [2,3] : [10,11]).map(id => ({id,position:'F6',unit_type:'Ship'})));
  state.map = structuredClone(fullMap);
  state.map.tiles = state.map.tiles.map(([position]) => [position, ['F6','G7'].includes(position) ? 'Water' : 'Fertile']);
  return state;
}
