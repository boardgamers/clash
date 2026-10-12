import { positionXY } from './model.ts';
import type { Game } from './types.ts';

/** The same public sea-space restriction as Game::is_pirate_zone. */
export function pirateBlockade(game: Pick<Game, 'map' | 'players'>): string[] {
  // Escorted Pirate Allies belong to Carthage, so they do not create a blockade.
  const pirates = game.players.find((p) => p.civilization === 'Pirates')?.units ?? [];
  const origins = [...new Set(pirates.map((u) => u.position))].map(positionXY);
  return game.map.tiles
    .filter(([position, terrain]) => {
      if (terrain !== 'Water') return false;
      const [x, z] = positionXY(position);
      return origins.some(([px, pz]) => (px - x) ** 2 + (pz - z) ** 2 <= 3.001);
    })
    .map(([position]) => position);
}
