import { resources, type Pile, type Resource } from './types.ts';

export const samePayment = (a: Pile, b: Pile) => resources.every((r) => (a[r] ?? 0) === (b[r] ?? 0));

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
