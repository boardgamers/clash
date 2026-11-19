import type { Terrain } from './types.ts';

/** Visual identity only: never use the unwrapped kind for action legality. */
export function terrainDisplay(terrain: Terrain) {
  const exhausted = typeof terrain !== 'string';
  const kind = exhausted ? terrain.Exhausted : terrain;
  return { kind, exhausted, prominent: kind === 'Forest' || kind === 'Mountain' };
}
