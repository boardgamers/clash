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

// Only engine-approved passenger moves are offered; the ship itself stays at sea.
export function passengerLandings(
  view: View | null,
  ships: number[],
  destinations: (ids: number[]) => MoveDestination[],
) {
  const passengers = (view?.units ?? []).filter((u) => u.carrier !== null && ships.includes(u.carrier));
  if (!passengers.length) return [];
  const groups = [passengers.map((u) => u.id), ...passengers.map((u) => [u.id])];
  const targets = new Map<string, number[]>();
  for (const group of groups)
    for (const d of destinations(group)) {
      if (d.terrain !== 'Water' && d.terrain !== 'Unexplored' && !targets.has(d.position))
        targets.set(d.position, group);
    }
  return [...targets].map(([position, units]) => ({ position, units }));
}
