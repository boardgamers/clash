import {
  BookOpen,
  Store,
  Landmark,
  Telescope,
  Castle,
  Anchor,
  Church,
  Footprints,
  Swords,
  Flag,
  Shield,
  Ship,
} from 'lucide-svelte';
import type { RecruitSelection, UnitKind } from './types';
import { actionReason } from './model';
export const buildingInfo: Record<string, { icon: typeof Landmark; effect: string }> = {
  Academy: { icon: BookOpen, effect: 'Gain 2 ideas when built.' },
  Market: { icon: Store, effect: 'Recruit cavalry and elephants here.' },
  Obelisk: { icon: Landmark, effect: 'This building cannot be culturally influenced.' },
  Observatory: { icon: Telescope, effect: 'Draw an action card when built.' },
  Fortress: { icon: Castle, effect: 'Defending: +1 die and cancel 1 hit in round 1.' },
  Port: { icon: Anchor, effect: 'Recruit ships. Collect gold or mood from the port tile.' },
  Temple: { icon: Church, effect: 'Gain 1 mood or culture when built.' },
};
export const unitInfo: Record<
  UnitKind,
  { icon: typeof Footprints; key: Exclude<keyof RecruitSelection, 'leader'>; effect: string }
> = {
  Settler: { icon: Footprints, key: 'settlers', effect: 'Exploration · Found cities' },
  Infantry: { icon: Swords, key: 'infantry', effect: '+1 combat value on infantry face · Tactics to move' },
  Cavalry: { icon: Flag, key: 'cavalry', effect: '+2 combat value on cavalry face · Requires Market' },
  Elephant: {
    icon: Shield,
    key: 'elephants',
    effect: 'Blocks 1 hit on elephant face · 0 die value · Requires Market',
  },
  Ship: {
    icon: Ship,
    key: 'ships',
    effect: 'Naval combat · Carries 2 land units · Requires Port',
  },
};
export function cityReason(reason: string | null, size = 1) {
  if (reason === 'Need more cities') return `Requires ${size + 1} cities to build here.`;
  if (reason === 'Invalid replacement')
    return 'Choose matching units on the map to replace pieces missing from your supply.';
  if (reason === 'Too many units') return 'The selection exceeds this city’s recruitment capacity.';
  return actionReason(reason).replace('Mising building:', 'Requires a');
}
