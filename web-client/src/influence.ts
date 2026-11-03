import type { MapPick, Pile, Session, View } from './types.ts';
import { structureKey } from './decision-controls.ts';
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

type Attempt = {
  selected_structure?: { position: string; structure: string | { Building: string } | { Wonder: string } };
  target_unit?: { player: number; unit: number } | null;
};
export const influenceKey = (offer: InfluenceOffer) => `${offer.position}/${offer.name}/${offer.variant}`;

export const influenceDestinationKey = (offer: InfluenceOffer) => `${offer.position}/${offer.name}`;
type InfluenceOrigin = NonNullable<InfluenceOffer['origins']>[number];

// Fair d6, success at 5+. Compare exact numerators over 36, including an optional
// reroll of a failed die. A reroll adds no benefit when success is already certain.
export function influenceSuccessScore(origin: InfluenceOrigin) {
  const successes = Math.max(0, Math.min(6, 2 + (origin.rollBonus ?? 0)));
  return origin.reroll ? 12 * successes - successes * successes : 6 * successes;
}
const canBoost = (offer: InfluenceOffer, origin: InfluenceOrigin) =>
  origin.position !== offer.position && !origin.preventBoost;
const successReward = (origin: InfluenceOrigin) => (origin.successRewards ?? [])
  .reduce<Pile>((total, reward) => influenceUpfrontCost(total, reward.resources), {});
const rewardSize = (origin: InfluenceOrigin) =>
  Object.values(successReward(origin)).reduce((sum, n) => sum + (n ?? 0), 0);
const compareBenefits = (offer: InfluenceOffer, a: InfluenceOrigin, b: InfluenceOrigin) =>
  influenceSuccessScore(b) - influenceSuccessScore(a) ||
  Number(canBoost(offer, b)) - Number(canBoost(offer, a)) || rewardSize(b) - rewardSize(a);

export function bestInfluenceOrigin(offer: InfluenceOffer) {
  const cost = (o: NonNullable<InfluenceOffer['origins']>[number]) =>
    Object.values(influenceUpfrontCost(o.actionPayment, o.payment)).reduce((sum, n) => sum + (n ?? 0), 0);
  const origins = offer.origins ?? [];
  const reclaiming = origins.some(o => o.position === offer.position);
  const boostableCities = reclaiming
    ? origins.filter(o => o.position !== offer.position && !o.settlers && !o.preventBoost)
    : [];
  return [...(boostableCities.length ? boostableCities : origins)]
    // Improve otherwise equal-cost attempts without buying a benefit. The existing
    // reclaim preference above is intentional; explicit choices bypass all defaults.
    .sort((a, b) => cost(a) - cost(b) || Number(!!b.free) - Number(!!a.free) || compareBenefits(offer, a, b))[0] ?? null;
}
export function influenceDestinations(offers: InfluenceOffer[]) {
  return offers.filter((o, i) => offers.findIndex(other => influenceDestinationKey(other) === influenceDestinationKey(o)) === i);
}

/** The board model an offer targets: a structure key or an army unit. */
export function influenceTarget(offer: InfluenceOffer) {
  const action = offer.action as { Playing?: { InfluenceCultureAttempt?: Attempt } } | null;
  const attempt = action?.Playing?.InfluenceCultureAttempt;
  if (attempt?.target_unit) return { kind: 'unit' as const, ...attempt.target_unit };
  const structure = attempt?.selected_structure?.structure;
  return structure ? { kind: 'structure' as const, structure: structureKey(structure) } : null;
}

/** Resolve a map click to one offer, or null when the hex has several candidates. */
export function influenceMapPick(offers: InfluenceOffer[], position: string, pick: MapPick) {
  const here = influenceDestinations(offers.filter((offer) => offer.position === position));
  if (pick.kind === 'unit' || pick.structure) {
    const exact = here.filter((offer) => {
      const target = influenceTarget(offer);
      return pick.kind === 'unit'
        ? target?.kind === 'unit' && target.player === pick.player && target.unit === pick.unit
        : target?.kind === 'structure' && target.structure === pick.structure;
    });
    if (exact.length) return exact.length === 1 ? exact[0] : null;
  }
  return here.length === 1 ? here[0] : null;
}

/** State of the dedicated cultural influence map mode, if it is active. */
export function activeInfluence(s: Session) {
  if (
    !s.influenceMode ||
    !s.abilitiesOpen ||
    s.mode !== 'overview' ||
    !s.view?.canPlay ||
    s.view.decision ||
    !s.view.influence?.length
  )
    return;
  const offers = s.view.influence;
  const target = offers.find((offer) => influenceKey(offer) === s.influenceTarget) ?? null;
  const origin =
    target?.origins?.find((o) => o.position === s.influenceOrigin) ??
    (target ? bestInfluenceOrigin(target) : null) ??
    null;
  const targets = [...new Set(offers.map((offer) => offer.position))];
  const position =
    target?.position ??
    (s.influencePosition && targets.includes(s.influencePosition) ? s.influencePosition : null);
  const source = target ? (origin?.position ?? target.origin) : null;
  // Once a target is chosen, its alternative source cities become selectable on the map.
  const origins = (target?.origins ?? []).map((o) => o.position).filter((p) => !targets.includes(p));
  return {
    offers,
    target,
    origin,
    source,
    position,
    targets,
    origins,
    positions: [...targets, ...origins],
    selected: [position, source].filter((p): p is string => !!p),
  };
}

/** Recommend savings only when they do not give up source benefits. */
function retainsInfluenceBenefits(offer: InfluenceOffer, candidate: InfluenceOrigin, selected: InfluenceOrigin) {
  const kept = successReward(candidate);
  return influenceSuccessScore(candidate) >= influenceSuccessScore(selected) &&
    (!canBoost(offer, selected) || canBoost(offer, candidate)) &&
    Object.entries(successReward(selected)).every(([resource, amount]) =>
      (kept[resource as keyof Pile] ?? 0) >= (amount ?? 0));
}

/** Recommend only an eligible city for this same target and action variant. */
export function cheaperInfluenceOrigin(
  offer: InfluenceOffer | null,
  selected: NonNullable<InfluenceOffer['origins']>[number] | null,
) {
  if (!offer) return null;
  const currentCost = (selected?.payment ?? offer.payment).culture_tokens ?? 0;
  return (
    (offer.origins ?? [])
      .filter(
        (from) =>
          !from.settlers &&
          (!selected || retainsInfluenceBenefits(offer, from, selected)) &&
          // Self-targeting loses the ability to boost, so it is not an equivalent saving.
          !(from.position === offer.position && selected?.position !== offer.position && !selected?.preventBoost) &&
          from.position !== (selected?.position ?? offer.origin) &&
          (from.payment.culture_tokens ?? 0) < currentCost,
      )
      .sort((a, b) => (a.payment.culture_tokens ?? 0) - (b.payment.culture_tokens ?? 0) || compareBenefits(offer, a, b))[0] ?? null
  );
}
