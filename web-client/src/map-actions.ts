import type { Game, MoveDestination, View } from './types';

export function canMoveOnMap(view: View | null, game: Game | null) {
  return (
    !!view &&
    !view.decision &&
    !view.explorationDecision &&
    !view.choiceDecision &&
    !view.objectiveDecision &&
    (!!view.stopMovement || (view.canPlay && (game?.actions_left ?? 0) > 0))
  );
}

export function moveOrigins(
  view: View | null,
  game: Game | null,
  target: string,
  destinations: (units: number[]) => MoveDestination[],
) {
  if (!canMoveOnMap(view, game)) return [];
  return (view?.units ?? []).filter(
    (unit) =>
      // Units aboard ships may disembark onto the tile they already occupy.
      (unit.position !== target || unit.carrier !== null) &&
      destinations([unit.id]).some((route) => route.position === target),
  );
}
