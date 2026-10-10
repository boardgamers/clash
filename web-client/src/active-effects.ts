import type { ActiveEffect, EventInfo } from './types.ts';
import type { CardRule } from './card-reference.ts';

export function effectSourceReference(effect: ActiveEffect, events: EventInfo[], cards: CardRule[]) {
  if (effect.source?.kind === 'Event') {
    const event = events.find((event) => event.id === effect.source?.id);
    return event ? { kind: 'event' as const, event } : null;
  }
  if (effect.source?.kind === 'Action card') {
    const matches = cards.filter((card) => card.name === effect.source?.name);
    const exact =
      effect.source.id == null ? undefined : matches.find((card) => card.id === effect.source?.id);
    // Legacy effects without an exact copy still expose the common action rules.
    // Never show another copy's different battle use.
    const card = exact ?? (matches.length > 1 ? { ...matches[0], tactics: null } : matches[0]);
    return card ? { kind: 'card' as const, card } : null;
  }
  return null;
}
