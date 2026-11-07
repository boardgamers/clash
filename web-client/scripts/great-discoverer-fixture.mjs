import { readFileSync } from 'node:fs';

// Synthetic engine fixture only; no production saves or network calls.
/** @param {any} engine @param {{ civilization?: string, irrigation?: boolean, resources?: Partial<Record<string, number>>, noCities?: boolean, mixedTerrain?: boolean }} options */
export function greatDiscovererFixture(engine, { civilization = 'Egypt', irrigation = true, resources = { food: 2 }, noCities = false, mixedTerrain = false } = {}) {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/great_persons/great_explorer.json', import.meta.url), 'utf8'));
  const player = game.players[1];
  player.civilization = civilization;
  if (irrigation) player.advances.push('Irrigation');
  player.resources = resources;
  player.action_cards = [118];
  if (noCities) player.destroyed_structures = { cities: 6 };
  if (mixedTerrain) game.map.unexplored_blocks.find(b => b.position.top_tile === 'B6').block.terrain = ['Fertile', 'Water', { Exhausted: 'Fertile' }, 'Barren'];
  let state = JSON.stringify(game);
  const apply = action => state = engine.tryMove(state, JSON.stringify(action), 1);
  const request = () => JSON.parse(state).events?.at(-1)?.handler?.request;
  const view = () => JSON.parse(engine.webView(engine.stripSecret(state, 1), 1));
  return {
    get state() { return state; },
    set state(value) { state = value; },
    apply, request, view,
    reveal() {
      apply({ Playing: { ActionCard: 118 } });
      const beforeExplore = JSON.parse(state);
      apply({ Response: { SelectPositions: ['B6'] } });
      if (request() === 'ExploreResolution') apply({ Response: { ExploreResolution: 0 } });
      return beforeExplore;
    },
  };
}
