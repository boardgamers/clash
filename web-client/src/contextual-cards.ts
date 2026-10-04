import type { ActionCard, Game, View } from './types';

export type CardContext = 'collect' | 'research' | 'build' | 'recruit' | 'happiness' | 'after-battle';
const suggestions: Record<string, { context: CardContext; benefit: string }> = {
  'Mass Production': { context: 'collect', benefit: '+2 tiles' },
  'Production Focus': { context: 'collect', benefit: 'Reuse tiles' },
  'Quick Advance': { context: 'research', benefit: 'Gain an advance' },
  Inspiration: { context: 'research', benefit: 'Copy a nearby advance' },
  Synergies: { context: 'research', benefit: 'Research 2 advances' },
  'Teach Us': { context: 'research', benefit: 'Gain a defeated opponent’s advance' },
  'Technology Trade': { context: 'research', benefit: 'Trade advances' },
  'New Ideas': { context: 'research', benefit: 'Research, then +2 ideas' },
  Ideas: { context: 'research', benefit: '+1 idea per Academy' },
  'Great Ideas': { context: 'research', benefit: '+2 ideas' },
  'City Development': { context: 'build', benefit: 'Build without resource cost' },
  Militia: { context: 'recruit', benefit: '+1 Infantry' },
  'Hero General': { context: 'happiness', benefit: '+1 mood step' },
};

export function contextualCards(view: View | null, context: CardContext) {
  if (!view?.canPlay) return [];
  return (view.actionCards ?? []).flatMap((card: ActionCard) => {
    const suggestion = suggestions[card.name];
    const matches =
      suggestion?.context === context ||
      (context === 'after-battle' && (card.name === 'Great Ideas' || card.name === 'Hero General'));
    return card.action && suggestion && matches ? [{ card, benefit: suggestion.benefit }] : [];
  });
}

export function activeCollectionCard(game: Game | null) {
  const effect = game?.permanent_effects?.find(
    (e): e is { Collect: string } => !!e && typeof e === 'object' && 'Collect' in e,
  );
  if (effect?.Collect === 'MassProduction') return 'Mass Production · +2 tiles';
  if (effect?.Collect === 'ProductionFocus') return 'Production Focus · Reuse tiles';
  return null;
}
