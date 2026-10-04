import type { RecruitSelection, UnitKind, View } from './types';

const quantities: Record<UnitKind, Exclude<keyof RecruitSelection, 'leader'>> = {
  Settler: 'settlers',
  Infantry: 'infantry',
  Cavalry: 'cavalry',
  Elephant: 'elephants',
  Ship: 'ships',
};

export function requiredRecruitDiscards(view: View | null, city: string | null, recruits: RecruitSelection) {
  return (view?.cityActions.find((c) => c.position === city)?.recruits ?? []).flatMap((item) => {
    const count = Math.max(0, (recruits[quantities[item.type]] ?? 0) - item.available);
    return count
      ? [{ type: item.type, count, units: (view?.units ?? []).filter((u) => u.type === item.type) }]
      : [];
  });
}

export function recruitLeaderToDiscard(view: View | null, recruits: RecruitSelection) {
  return recruits.leader ? view?.units?.find((u) => typeof u.type === 'object') : undefined;
}

export function recruitDiscardSelection(
  view: View | null,
  city: string | null,
  recruits: RecruitSelection,
  selected: number[],
) {
  const ordinary = requiredRecruitDiscards(view, city, recruits).flatMap((group) =>
    [...new Set(selected)].filter((id) => group.units.some((u) => u.id === id)).slice(0, group.count),
  );
  // Choosing a new leader necessarily retires the one already on the board.
  // The UI explains this before the user confirms the Recruit action.
  const leader = recruitLeaderToDiscard(view, recruits);
  return leader ? [...ordinary, leader.id] : ordinary;
}
