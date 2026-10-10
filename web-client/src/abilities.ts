import type { Session, View } from './types.ts';

type Offer = NonNullable<View['specialActions']>[number];
export function abilityKey(offer: Offer) {
  // A city-bound action is one ability with several legal targets.
  return JSON.stringify(offer.action, (key, value) => (key === 'city' ? undefined : value));
}
export function groupAbilities(offers: Offer[] = []) {
  const groups = new Map<string, { key: string; offer: Offer; offers: Offer[] }>();
  for (const offer of offers) {
    const key = abilityKey(offer);
    const group = groups.get(key);
    if (group) group.offers.push(offer);
    else groups.set(key, { key, offer, offers: [offer] });
  }
  return [...groups.values()];
}
export function activeCityAbility(s: Session) {
  if (!s.abilitiesOpen || s.mode !== 'overview' || !s.view?.canPlay || s.view.decision) return;
  return groupAbilities(s.view.specialActions).find(
    (group) => group.key === s.abilityChoice && group.offers.every((offer) => offer.position),
  );
}


export function abilityReason(reason?: string | null) {
  if (reason === 'Custom action cannot be played' || reason === 'Custom action not available') return 'Unavailable';
  if (reason === 'Not enough resources for action type') return 'Not enough resources';
  return reason ?? '';
}
