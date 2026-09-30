import type { Decision, View } from './types.ts';

export const researchDecision = (view: View | null | undefined) => !!view?.decision?.advanceSelection;

// A tile must identify exactly one choice. Unit and building requests can have
// several choices at the same position and retain their individual controls.
export function mapDecisionOptions(decision?: Decision | null) {
  if (!decision?.options.length || decision.fields.length) return [];
  const options = decision.options;
  return options.every((o) => o.position && o.value === o.position) &&
    new Set(options.map((o) => o.position)).size === options.length
    ? options
    : [];
}

export function toggleDecisionSelection(decision: Decision, selected: number[], index: number) {
  if (!decision.options[index]) return selected;
  if (selected.includes(index)) return selected.filter((i) => i !== index);
  if (decision.max === 1) return [index];
  return selected.length < decision.max ? [...selected, index] : selected;
}
