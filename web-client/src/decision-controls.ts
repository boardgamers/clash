import type { Decision, Game, MapPick, View } from './types.ts';

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

type StructureTarget = string | { Building: string } | { Wonder: string };

/** Stable key shared by decision options, influence offers and board models. */
export function structureKey(structure: StructureTarget) {
  if (typeof structure === 'string') return structure;
  return 'Building' in structure ? `Building:${structure.Building}` : `Wonder:${structure.Wonder}`;
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
  if (pick.structure) {
    // A clicked building model chooses exactly that structure.
    const exact = options.findIndex(
      (o) =>
        o.position === position &&
        o.mapTarget?.kind === 'structure' &&
        structureKey(o.mapTarget.structure) === pick.structure,
    );
    if (exact >= 0) return exact;
  }
  const matches = options.flatMap((o, index) => (o.position === position ? [index] : []));
  // A tile or stack click must never silently choose one of several pieces.
  return matches.length === 1 ? matches[0] : -1;
}

export function toggleDecisionSelection(decision: Decision, selected: number[], index: number) {
  if (!decision.options[index]) return selected;
  if (selected.includes(index)) return selected.filter((i) => i !== index);
  if (decision.max === 1) return [index];
  return selected.length < decision.max ? [...selected, index] : selected;
}

/** Cancel an initial ability prompt or refund a pending government-change payment. */
export function canCancelAbility(game: Game | null, view: View | null): boolean {
  if (!game || !view?.decision || !view.canUndo) return false;
  const governmentChoice = view.decision.options.some((option) =>
    option.value && typeof option.value === 'object' && 'new_government' in option.value);
  if (governmentChoice) return true; // Undo refunds the optional government-change payment.
  const actions = game.log?.at(-1)?.rounds.at(-1)?.turns.at(-1)?.actions;
  const action = actions?.[game.log_index - 1]?.action;
  return !!(action && typeof action === 'object' && 'Playing' in action &&
    typeof action.Playing === 'object' && action.Playing !== null && 'Custom' in action.Playing);
}
