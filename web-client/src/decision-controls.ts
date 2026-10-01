import type { Decision, MapPick, View } from './types.ts';

export const researchDecision = (view: View | null | undefined) => !!view?.decision?.advanceSelection;

export function mapDecisionOptions(decision?: Decision | null) {
  if (!decision?.options.length || decision.fields.length) return [];
  const options = decision.options;
  return options.every(
    (o) =>
      o.position &&
      (o.mapTarget ||
        (o.value === o.position && options.filter((other) => other.position === o.position).length === 1)),
  )
    ? options
    : [];
}

export function mapDecisionIndex(decision: Decision, position: string, pick: MapPick) {
  const options = mapDecisionOptions(decision);
  if (pick.kind === 'decision') {
    const index = pick.decisionIndex ?? -1;
    return options[index]?.position === position ? index : -1;
  }
  if (pick.kind === 'unit') {
    return options.findIndex(
      (o) =>
        o.position === position &&
        (!o.mapTarget ||
          (o.mapTarget.kind === 'unit' &&
            o.mapTarget.player === pick.player &&
            o.mapTarget.unit === pick.unit)),
    );
  }
  const matches = options.flatMap((o, index) => (o.position === position ? [index] : []));
  // A tile or stack click must never silently choose one of several pieces.
  return matches.length === 1 && !options[matches[0]].mapTarget ? matches[0] : -1;
}

export function toggleDecisionSelection(decision: Decision, selected: number[], index: number) {
  if (!decision.options[index]) return selected;
  if (selected.includes(index)) return selected.filter((i) => i !== index);
  if (decision.max === 1) return [index];
  return selected.length < decision.max ? [...selected, index] : selected;
}
