import { resourceNames, type Choice, type Pile, type Resource, type Selection } from './types.ts';

export function sameCollection(a: Choice, b: Choice) {
  return a.position === b.position && JSON.stringify(a.pile) === JSON.stringify(b.pile);
}

export function collectionYield(choice: Choice, selection: Selection[] = []): Pile {
  const selected = selection.find((c) => sameCollection(c, choice));
  const pile: Pile = {};
  for (const [resource, amount] of Object.entries(choice.pile))
    pile[resource as Resource] = amount! * (selected?.times ?? 1);
  for (const bonus of choice.bonuses ?? []) {
    const eligible = selection.filter((c) => c.bonuses?.some((b) => b.source === bonus.source));
    const index = selected ? eligible.indexOf(selected) : eligible.length;
    if (index >= bonus.limit) continue;
    for (const [resource, amount] of Object.entries(bonus.pile))
      pile[resource as Resource] = (pile[resource as Resource] ?? 0) + amount!;
  }
  return pile;
}

export function collectionBonusLabel(choice: Choice) {
  return (choice.bonuses ?? [])
    .map(
      (b) =>
        `${Object.entries(b.pile)
          .map(([r, n]) => `+${n} ${resourceNames[r as Resource].toLowerCase()}`)
          .join(' ')} · ${b.source} (up to ${b.limit} tiles)`,
    )
    .join(', ');
}
