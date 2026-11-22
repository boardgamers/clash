import type { Terrain } from './types.ts';

/** Top of the hex slab, excluding trees, mountains, cities and pieces. */
export function tileSurfaceHeight(terrain: Terrain) {
  const { kind } = terrainDisplay(terrain);
  return (kind === 'Water' ? 0.13 : kind === 'Unexplored' ? 0.27 : 0.43) - 0.12;
}

/** Visual identity only: never use the unwrapped kind for action legality. */
export function terrainDisplay(terrain: Terrain) {
  const exhausted = typeof terrain !== 'string';
  const kind = exhausted ? terrain.Exhausted : terrain;
  return { kind, exhausted, prominent: kind === 'Forest' || kind === 'Mountain' };
}
