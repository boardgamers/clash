import type { BoardFrame, LoggedAction } from './types.ts';

export function influenceResult(text: string): 'succeeded' | 'failed' | null {
  if (/Cultural influence succeeded|pays to increase the dice roll/.test(text)) return 'succeeded';
  if (
    /Cultural influence failed|Declined to pay to increase the dice roll|does not have enough resources to increase the roll/.test(
      text,
    )
  )
    return 'failed';
  return null;
}

/** Use only public actions up to this frame; a later boost must not rewrite the original roll. */
export function influenceRecap(
  history: LoggedAction[],
  start: number,
  before: BoardFrame,
  frame: BoardFrame,
) {
  const attemptIn = (action: LoggedAction) => {
    const entry = action.items?.find((i) => i.InfluenceCultureAttempt);
    if (entry?.InfluenceCultureAttempt) return { player: entry.player, ...entry.InfluenceCultureAttempt };
    const playing = typeof action.action === 'object' ? action.action?.Playing : undefined;
    const attempt =
      playing && typeof playing === 'object' && 'InfluenceCultureAttempt' in playing
        ? (playing.InfluenceCultureAttempt as {
            selected_structure: { position: string; structure: unknown };
            target_unit?: { player: number; unit: number };
            starting_position?: string;
          })
        : undefined;
    return attempt?.selected_structure
      ? {
          player: action.player ?? frame.actor,
          ...attempt.selected_structure,
          starting_city_position: attempt.starting_position,
          target_unit: attempt.target_unit,
          target_player: attempt.target_unit?.player,
        }
      : undefined;
  };
  const attempts = history.map(attemptIn);
  const backwards = [...attempts].reverse().findIndex(Boolean);
  if (backwards < 0) return null;
  const index = attempts.length - 1 - backwards;
  const attempt = attempts[index]!;
  const current = history.slice(Math.max(start, index));
  const messages = current.flatMap((a) => [
    ...(a.items ?? []).flatMap((i) => (i.Text ? [i.Text] : [])),
    ...(a.log ?? []),
  ]);
  const success = messages.some((t) => influenceResult(t) === 'succeeded');
  const failure = messages.some((t) => influenceResult(t) === 'failed');
  const pending = messages.some((t) => /now has the option to pay .*increase the dice roll/.test(t));
  if (!success && !failure && !pending && index < start) return null;
  const cityOwner = before.players.find((p) => p.cities?.some((c) => c.position === attempt.position));
  const city = cityOwner?.cities?.find((c) => c.position === attempt.position);
  const structure = attempt.structure;
  const building =
    structure &&
    typeof structure === 'object' &&
    'Building' in structure &&
    typeof structure.Building === 'string'
      ? structure.Building
      : undefined;
  const unit = attempt.target_unit;
  const unitType =
    unit &&
    before.players.find((p) => p.id === unit.player)?.units?.find((u) => u.id === unit.unit)?.unit_type;
  const name = unit
    ? typeof unitType === 'string'
      ? unitType
      : 'army unit'
    : (building ?? (structure === 'CityCenter' ? 'Settlement' : 'building'));
  const owner =
    unit?.player ??
    (building
      ? city?.city_pieces?.[
          building.toLowerCase() as keyof NonNullable<NonNullable<typeof city>['city_pieces']>
        ]
      : cityOwner?.id) ??
    attempt.target_player;
  const target = before.players.find((p) => p.id === owner)?.civilization;
  return {
    actor: before.players.find((p) => p.id === attempt.player)?.civilization,
    description: `Cultural influence${success ? ' succeeded' : failure ? ' failed' : ''} against ${target ? `${target}’s ` : ''}${name}${!success && !failure ? (pending ? ' · Awaiting boost' : ' · Awaiting result') : ''}`,
    positions: [attempt.position, attempt.starting_city_position].filter((p): p is string => !!p),
  };
}
