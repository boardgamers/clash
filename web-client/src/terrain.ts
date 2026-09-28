import { Compass, LandPlot, Mountain, Trees, Waves, Wheat } from 'lucide-svelte';
import type { Terrain } from './types';

const terrains = {
  Forest: { Icon: Trees, color: '#82a67d' },
  Fertile: { Icon: Wheat, color: '#ddcf85' },
  Mountain: { Icon: Mountain, color: '#b6c0b8' },
  Water: { Icon: Waves, color: '#99c5cc' },
  Barren: { Icon: LandPlot, color: '#dbc39e' },
  Unexplored: { Icon: Compass, color: '#a6b9b3' },
};

export function terrainInfo(terrain: Terrain) {
  const exhausted = typeof terrain !== 'string';
  const name = exhausted ? terrain.Exhausted : terrain;
  return {
    ...(terrains[name as keyof typeof terrains] ?? terrains.Barren),
    label: exhausted ? `Exhausted ${name.toLowerCase()}` : name,
    exhausted,
  };
}
