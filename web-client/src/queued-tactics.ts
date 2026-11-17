import type { Decision, View } from './types';

export type TacticsQueue = NonNullable<View['earlyChoices']>[number];

/** Adapt only owner-authorized offer IDs to the normal hand-card decision UI. */
export function queuedTacticsDecision(view: View | null, offer: TacticsQueue | undefined): Decision | null {
  if (!view || !offer) return null;
  return {
    reward: false,
    endOfAge: false,
    name: 'Tactics',
    description: '',
    min: 0,
    max: 1,
    fields: [],
    tacticsSelection: true,
    options: offer.options.flatMap((option) => {
      const cards = (option.response as { SelectHandCards?: { ActionCard?: number }[] }).SelectHandCards;
      const id = cards?.[0]?.ActionCard;
      if (!id || cards?.length !== 1) return [];
      const card = view.actionCards?.find((card) => card.id === id);
      return card?.tactics
        ? [
            {
              value: { ActionCard: id },
              name: card.name,
              description: card.description,
              position: null,
              card: {
                kind: 'action' as const,
                name: card.name,
                description: card.description,
                tactics: card.tactics,
              },
            },
          ]
        : [];
    }),
  };
}
