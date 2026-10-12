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
  return paymentWithAmount(choices, current, resource, amount, 'food', false);
}

// Keep the requested amount exact and choose a complete legal payment. Prefer
// adjusting the preferred substitute before changing the other chosen resources.
export function paymentWithAmount(choices: Pile[], current: Pile, resource: Resource, amount: number, preferred: Resource = 'gold', avoidOverpayment = true): Pile {
  let selected = current;
  let distance = Infinity;
  const candidates = choices.filter((choice) => (choice[resource] ?? 0) === amount);
  // Honor the edited amount first. For costs, discard candidates that add
  // unnecessary resources before preferring the smallest change to other fields.
  for (const choice of avoidOverpayment ? withoutOverpayments(candidates) : candidates) {
    const change = resources.reduce(
      (sum, r) => sum + Math.abs((choice[r] ?? 0) - (current[r] ?? 0)) * (r === preferred ? 1 : 2),
      0,
    );
    if (change < distance) {
      selected = choice;
      distance = change;
    }
  }
  return selected;
}

// Substitutions can leave valid but needlessly expensive payments
// (e.g. 1 gold + 1 wood when Mercenaries allows the same ship for 1 gold).
function withoutOverpayments(choices: Pile[]): Pile[] {
  return choices.filter((choice) => !choices.some((other) =>
    resources.every((r) => (other[r] ?? 0) <= (choice[r] ?? 0)) &&
    resources.some((r) => (other[r] ?? 0) < (choice[r] ?? 0)),
  ));
}
