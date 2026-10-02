import {
  Landmark,
  Flower2,
  Pyramid,
  TreePine,
  Sun,
  Ship,
  Flame,
  Swords,
  Sunrise,
  Crown,
  Anchor,
  Castle,
} from 'lucide-svelte';
import vikings from '../../client/assets/viking-ship-svgrepo-com.png';
import rome from '../../client/assets/colosseum-rome-svgrepo-com.png';
import greece from '../../client/assets/temple-building-with-columns-svgrepo-com.png';
import china from '../../client/assets/great-wall-of-china-chinese-svgrepo-com.png';
import pirates from '../../client/assets/pirate-symbol-mark-svgrepo-com.png';
import barbarians from '../../client/assets/warrior-svgrepo-com.png';

const icons: Record<string, typeof Landmark> = {
  India: Flower2,
  Babylonia: Castle,
  Aztecs: Flame,
  Carthage: Ship,
  Celts: TreePine,
  Egypt: Pyramid,
  Huns: Swords,
  Japan: Sun,
  Maya: Sunrise,
  Persia: Crown,
  Phoenicia: Anchor,
};
export const civilizationIcon = (civilization: string) => icons[civilization] ?? Landmark;
export const civilizationImages: Record<string, string> = {
  Vikings: vikings,
  Rome: rome,
  Greece: greece,
  China: china,
  Pirates: pirates,
  Barbarians: barbarians,
};
