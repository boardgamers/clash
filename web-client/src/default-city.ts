import type { View } from './types.ts';

export type CityAction = 'collect' | 'build' | 'recruit';
export interface CollectionPotential {
  position: string;
  amount: number;
}

export function defaultCity(
  view: View | null,
  action: CityAction,
  current: string | null,
  potentials: CollectionPotential[] = [],
): string | null {
  if (!view?.cities.length) return null;
  const amounts = new Map(potentials.map((city) => [city.position, city.amount]));
  const eligible = view.cities.filter((city) => {
    if (action === 'collect') return !city.reason && (amounts.get(city.position) ?? 0) > 0;
    const offers = view.cityActions.find((offers) => offers.position === city.position);
    return action === 'build'
      ? offers?.buildings.some((building) => !building.owned && building.choices.length > 0)
      : offers?.recruits.some((unit) => !unit.reason && (unit.limit ?? unit.available) > 0) ||
          offers?.leaders?.some((leader) => !leader.reason);
  });
  // When only browsing, retain the inspected city. Action availability may be
  // empty outside our turn, but the dialog must still be useful as a reference.
  if (!eligible.length)
    return view.cities.find((city) => city.position === current)?.position ?? view.cities[0].position;
  return eligible.sort(
    (a, b) =>
      Number(a.activations > 0) - Number(b.activations > 0) ||
      (action === 'collect' ? amounts.get(b.position)! - amounts.get(a.position)! : 0) ||
      b.size - a.size ||
      Number(b.position === current) - Number(a.position === current),
  )[0].position;
}
