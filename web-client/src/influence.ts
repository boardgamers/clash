import type { Pile, View } from './types.ts';
export type InfluenceOffer = NonNullable<View['influence']>[number];
export function influenceUpfrontCost(actionPayment: Pile = {}, rangePayment: Pile = {}) {
  const total: Pile = { ...actionPayment };
  for (const [resource, amount] of Object.entries(rangePayment)) {
    const key = resource as keyof Pile;
    total[key] = (total[key] ?? 0) + (amount ?? 0);
  }
  return total;
}
export function influencePaymentMatches(view: View, payment: Pile | null) {
  const d = view.decision,
    field = d?.fields[0];
  if (
    !payment ||
    view.influenceContext?.stage !== 'range' ||
    !d ||
    d.reward ||
    d.options.length ||
    d.fields.length !== 1 ||
    field?.optional ||
    field?.choices?.length !== 1
  )
    return false;
  return [...new Set([...Object.keys(payment), ...Object.keys(field.choices[0])])].every((resource) => {
    const key = resource as keyof Pile;
    return (payment[key] ?? 0) === (field.choices![0][key] ?? 0);
  });
}
