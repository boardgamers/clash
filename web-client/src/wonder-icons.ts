import { Landmark, Mountain, Trees, LibraryBig, TowerControl, Castle, Theater, Crown } from 'lucide-svelte';

export const wonderIcons: Record<string, typeof Landmark> = {
  Pyramids: Mountain,
  GreatGardens: Trees,
  GreatLibrary: LibraryBig,
  GreatLighthouse: TowerControl,
  GreatWall: Castle,
  Colosseum: Theater,
  GreatStatue: Crown,
  GreatMausoleum: Landmark,
};
