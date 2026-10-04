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
import { resourceNames, type Pile, type RecruitSelection, type Resource, type UnitKind } from './types';
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
  {
    icon: typeof Footprints;
    key: Exclude<keyof RecruitSelection, 'leader'>;
    effect: string;
    requirement?: { building: string; advance: string };
  }
> = {
  Settler: { icon: Footprints, key: 'settlers', effect: 'Exploration · Found cities' },
  Infantry: { icon: Swords, key: 'infantry', effect: '+1 combat value on infantry face · Tactics to move' },
  Cavalry: {
    icon: Flag,
    key: 'cavalry',
    effect: '+2 combat value on cavalry face',
    requirement: { building: 'Market', advance: 'Bartering' },
  },
  Elephant: {
    icon: Shield,
    key: 'elephants',
    effect: 'Blocks 1 hit on elephant face · 0 die value',
    requirement: { building: 'Market', advance: 'Bartering' },
  },
  Ship: {
    icon: Ship,
    key: 'ships',
    effect: 'Naval combat · Carries 2 land units',
    requirement: { building: 'Port', advance: 'Fishing' },
  },
};
export function sameRecruitPayment(a: Pile, b: Pile) {
  return (Object.keys(resourceNames) as Resource[]).every((r) => (a[r] ?? 0) === (b[r] ?? 0));
}
export function recruitCostOptions(sources: string[] = [], payment: Pile, standard: Pile) {
  const notes = sources.map((source) =>
    source === 'Sanitation'
      ? 'Sanitation: 1 settler per recruitment'
      : source === 'Draft'
        ? 'Draft: mood tokens pay for only 1 infantry per Recruit action. Additional infantry use normal costs.'
        : source,
  );
  if ((payment.gold ?? 0) > (standard.gold ?? 0)) notes.push('Gold can replace resources');
  return notes.join(' · ');
}
export function cityReason(reason: string | null, size = 1) {
  if (reason === 'Need more cities') return `Requires ${size + 1} cities to build here.`;
  if (reason === 'Invalid replacement')
    return 'Choose units to discard from the board to free up pieces for these recruits.';
  if (reason === 'Too many units') return 'The selection exceeds this city’s recruitment capacity.';
  return actionReason(reason).replace('Mising building:', 'Requires a');
}
