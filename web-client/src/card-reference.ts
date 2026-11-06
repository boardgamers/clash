import type { ActionCard, Decision, View } from './types.ts';
export type CardRule = Pick<ActionCard, 'id' | 'name' | 'description' | 'free' | 'tactics'>;
export interface CardReference {
  name: string;
  card: CardRule;
}
export function cardReferences(view: View | null | undefined): CardReference[] {
  const catalog = view?.cardCatalog ?? [];
  const refs = new Map(catalog.map((card) => [card.name.toLowerCase(), { name: card.name, card }]));
  for (const card of catalog) {
    if (card.tactics && !refs.has(card.tactics.name.toLowerCase())) {
      refs.set(card.tactics.name.toLowerCase(), { name: card.tactics.name, card });
    }
  }
  return [...refs.values()];
}

/** Resolve only the public declaration, never infer a card from an opponent's hand or title. */
export function pendingActionCard(view: View | null | undefined, decision: Decision): CardRule | null {
  return view?.cardCatalog?.find((card) => card.id === decision.pendingActionCard) ?? null;
}
