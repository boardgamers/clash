import {
  resourceNames,
  type Choice,
  type CityView,
  type CollectionBonus,
  type Pile,
  type Resource,
  type Selection,
} from './types.ts';

export function sameCollection(a: Choice, b: Choice) {
  return a.position === b.position && JSON.stringify(a.pile) === JSON.stringify(b.pile);
}

export function collectionYield(choice: Choice, selection: Selection[] = []): Pile {
  const selected = selection.find((c) => sameCollection(c, choice));
  const pile: Pile = {};
  for (const [resource, amount] of Object.entries(choice.pile))
    pile[resource as Resource] = amount! * (selected?.times ?? 1);
  for (const bonus of choice.bonuses ?? []) {
    if (bonus.condition === 'exactlyOneFood') {
      const food =
        selection.reduce((total, tile) => total + (tile.pile.food ?? 0) * tile.times, 0) +
        (selected ? 0 : (choice.pile.food ?? 0));
      if (food !== 1) continue;
    }
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
          .join(
            ' ',
          )} · ${b.source} (${b.condition === 'exactlyOneFood' ? 'when collecting exactly 1 food' : `up to ${b.limit} tiles`})`,
    )
    .join(', ');
}

function collectionTotal(choices: Selection[], context = choices): Pile {
  const result: Pile = {};
  for (const c of choices)
    for (const [resource, amount] of Object.entries(collectionYield(c, context)))
      result[resource as Resource] = (result[resource as Resource] ?? 0) + amount!;
  return result;
}

function nextSelection(choice: Choice, selection: Selection[], city: CityView, extraCapacity: number) {
  const selected = selection.find((c) => sameCollection(c, choice));
  const used = selection.reduce((sum, c) => sum + c.times, 0);
  const capacity = city.capacity + extraCapacity;
  const next = selected
    ? selected.times < city.maxPerTile && used < capacity
      ? selection.map((c) => (c === selected ? { ...c, times: c.times + 1 } : c))
      : selection.filter((c) => c !== selected)
    : [...selection, { ...choice, times: 1 }];
  const nextIsValid =
    next.reduce((sum, c) => sum + c.times, 0) <= capacity &&
    next.filter((c) => c.position === choice.position).reduce((sum, c) => sum + c.times, 0) <=
      city.maxPerTile;
  return nextIsValid ? next : null;
}

// Show an action bonus on the choice that reaches its threshold, not on every
// selected tile. Before that choice is selected, preview what its next click adds.
export function collectionBonusIndicators(
  choice: Choice,
  selection: Selection[],
  city: CityView,
  extraCapacity = 0,
): CollectionBonus[] {
  if (city.reason || !city.collectionBonuses?.length) return [];
  const reaches = (pile: Pile, bonus: CollectionBonus) =>
    Object.entries(bonus.minimum).every(([r, n]) => (pile[r as Resource] ?? 0) >= n!);
  const selected = selection.find((c) => sameCollection(c, choice));
  const next = nextSelection(choice, selection, city, extraCapacity);
  const current = collectionTotal(selection);
  return city.collectionBonuses.filter((bonus) => {
    if (reaches(current, bonus)) {
      if (!selected) return false;
      const index = selection.indexOf(selected);
      return (
        !reaches(collectionTotal(selection.slice(0, index)), bonus) &&
        reaches(collectionTotal(selection.slice(0, index + 1)), bonus)
      );
    }
    return !!next && reaches(collectionTotal(next), bonus);
  });
}

export function collectionStorageWaste(
  choice: Choice,
  selection: Selection[],
  city: CityView,
  stock: Pile = {},
  limits: Pile = {},
  extraCapacity = 0,
): Pile {
  if (city.reason) return {};
  const index = selection.findIndex((c) => sameCollection(c, choice));
  const before = index < 0 ? selection : selection.slice(0, index);
  const after =
    index < 0 ? nextSelection(choice, selection, city, extraCapacity) : selection.slice(0, index + 1);
  if (!after) return {};
  const previous = collectionTotal(before, index < 0 ? before : selection);
  const next = collectionTotal(after, index < 0 ? after : selection);
  const waste: Pile = {};
  for (const [key, limit] of Object.entries(limits)) {
    const resource = key as Resource;
    const overflow = (collected: number) => Math.max(0, (stock[resource] ?? 0) + collected - limit!);
    const lost = overflow(next[resource] ?? 0) - overflow(previous[resource] ?? 0);
    if (lost > 0) waste[resource] = lost;
  }
  return waste;
}

export function collectionTriggerLabel(bonus: CollectionBonus) {
  const describe = (pile: Pile) =>
    Object.entries(pile)
      .map(([r, n]) => `${n} ${resourceNames[r as Resource].toLowerCase()}`)
      .join(' + ');
  return `${bonus.source}: +${describe(bonus.pile)} when collecting at least ${describe(bonus.minimum)}`;
}
