import { resources, type Pile, type Resource } from './types.ts';

export const samePayment = (a: Pile, b: Pile) => resources.every((r) => (a[r] ?? 0) === (b[r] ?? 0));

// A partial reward must still fit a legal complete choice without reducing any
// of the resources the player already selected.
export function canCompletePayment(choices: Pile[], current: Pile): boolean {
  return choices.some((choice) =>
    resources.every((r) => (current[r] ?? 0) >= 0 && (current[r] ?? 0) <= (choice[r] ?? 0)),
  );
}

export function rewardWithAmount(choices: Pile[], current: Pile, resource: Resource, amount: number): Pile {
  const next = { ...current, [resource]: amount };
  return Number.isInteger(amount) && canCompletePayment(choices, next) ? next : current;
}

// Keep the requested amount exact and choose a complete legal payment. Prefer
// adjusting gold substitutions over changing the other chosen resources.
export function paymentWithAmount(choices: Pile[], current: Pile, resource: Resource, amount: number): Pile {
  let selected = current;
  let distance = Infinity;
  for (const choice of choices) {
    if ((choice[resource] ?? 0) !== amount) continue;
    const change = resources.reduce(
      (sum, r) => sum + Math.abs((choice[r] ?? 0) - (current[r] ?? 0)) * (r === 'gold' ? 1 : 2),
      0,
    );
    if (change < distance) {
      selected = choice;
      distance = change;
    }
  }
  return selected;
}
