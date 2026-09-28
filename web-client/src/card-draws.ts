import type { CardDraw, Game, View } from './types.ts';

// The engine keeps redo entries in the last turn. Count only its applied actions.
function progress(game: Game) {
  const turns = (game.log ?? []).flatMap((age) => age.rounds.flatMap((round) => round.turns));
  return turns.reduce(
    (sum, turn, i) => sum + (i === turns.length - 1 ? game.log_index : (turn.actions?.length ?? 0)),
    0,
  );
}

export class CardDrawTracker {
  private previous: { seat: number | undefined; game: Game; view: View } | null = null;

  reset() {
    this.previous = null;
  }

  update(seat: number | undefined, game: Game, view: View): CardDraw[] {
    const before = this.previous;
    this.previous = { seat, game, view };
    if (!before || seat === undefined || before.seat !== seat || progress(game) <= progress(before.game))
      return [];
    return [
      ...view.wonderCards
        .filter((card) => !before.view.wonderCards.some((old) => old.id === card.id))
        .map((card) => ({ kind: 'wonder' as const, card })),
      ...view.objectiveCards
        .filter((card) => !before.view.objectiveCards.some((old) => old.id === card.id))
        .map((card) => ({ kind: 'objective' as const, card })),
    ];
  }
}
