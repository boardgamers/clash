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
  const here = offers.filter((offer) => offer.position === position);
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
    target?.origins?.find((o) => o.position === (s.influenceOrigin ?? target.origin)) ??
    target?.origins?.[0] ??
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
          from.position !== (selected?.position ?? offer.origin) &&
          (from.payment.culture_tokens ?? 0) < currentCost,
      )
      .sort((a, b) => (a.payment.culture_tokens ?? 0) - (b.payment.culture_tokens ?? 0))[0] ?? null
  );
}
