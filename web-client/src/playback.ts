import type { BoardFrame, Game } from './types.ts';

export function recapStart(game: Game, seat: number | undefined, seen = 0): number | null {
  const frames = game.board_history?.frames ?? [];
  if (seat === undefined || frames.length < 2 || seen >= frames.at(-1)!.cursor) return null;
  const lastTurn = lastIndex(frames, (f) => f.actor === seat && f.ended_turn);
  if (lastTurn < 0 || frames.slice(lastTurn + 1).some((f) => f.actor === seat)) return null;
  const start = lastIndex(frames, (f) => f.cursor <= Math.max(seen, frames[lastTurn].cursor));
  return start >= 0 && start < frames.length - 1 ? start : null;
}
export function frameAt(frames: BoardFrame[], cursor: number): number {
  return Math.max(
    0,
    lastIndex(frames, (frame) => frame.cursor <= cursor),
  );
}

/** Include the position before the latest opponent turn so its first move can animate. */
export function lastOpponentTurn(game: Game, seat: number | undefined) {
  const frames = game.board_history?.frames ?? [];
  if (seat === undefined) return null;
  let end = frames.length - 1;
  while (end > 0) {
    const boundary = lastIndex(frames.slice(0, end), (f) => f.ended_turn);
    const owner = frames[end].ended_turn
      ? frames[end].actor
      : frames.slice(boundary + 1, end + 1).find((f) => f.actor !== null)?.actor;
    const start = Math.max(0, boundary);
    if (owner != null && owner !== seat && start < end) return { start, end };
    end = boundary;
  }
  return null;
}

/** Describe and highlight only changes present in public board snapshots. */
export function frameDetails(before: BoardFrame | undefined, frame: BoardFrame | null) {
  if (!frame) return { caption: 'No recorded positions yet.', positions: [] as string[] };
  const actor = frame.players.find((p) => p.id === frame.actor)?.civilization;
  const positions = new Set<string>();
  const units = (f: BoardFrame | undefined) =>
    new Map(
      (f?.players ?? []).flatMap((p) =>
        (p.units ?? []).flatMap((u) =>
          [u, ...(u.carried_units ?? []).map((c) => ({ ...c, position: u.position }))].map(
            (unit) => [`${p.id}:${unit.id}`, unit] as const,
          ),
        ),
      ),
    );
  const oldUnits = units(before),
    newUnits = units(frame);
  if (before) {
    for (const [id, unit] of newUnits) {
      const old = oldUnits.get(id);
      if (!old || old.position !== unit.position) {
        positions.add(unit.position);
        if (old) positions.add(old.position);
      }
    }
    for (const [id, unit] of oldUnits) if (!newUnits.has(id)) positions.add(unit.position);
    const cities = (f: BoardFrame) =>
      new Map(
        f.players.flatMap((p) =>
          (p.cities ?? []).map((city) => [city.position, { owner: p.id, city }] as const),
        ),
      );
    const oldCities = cities(before),
      newCities = cities(frame);
    for (const position of new Set([...oldCities.keys(), ...newCities.keys()]))
      if (JSON.stringify(oldCities.get(position)) !== JSON.stringify(newCities.get(position)))
        positions.add(position);
    const tiles = new Map(before.tiles);
    for (const [position, terrain] of frame.tiles)
      if (JSON.stringify(tiles.get(position)) !== JSON.stringify(terrain)) positions.add(position);
  }
  let description = frame.title;
  if (before && frame.title === 'Recruit') {
    const counts = new Map<string, number>();
    for (const [id, unit] of newUnits) {
      if (oldUnits.has(id) || !id.startsWith(`${frame.actor}:`)) continue;
      const name = typeof unit.unit_type === 'string' ? unit.unit_type.toLowerCase() : 'leader';
      counts.set(name, (counts.get(name) ?? 0) + 1);
    }
    if (counts.size)
      description =
        'recruited ' +
        [...counts]
          .map(
            ([name, count]) =>
              `${count} ${name}${count > 1 && !['infantry', 'cavalry'].includes(name) ? 's' : ''}`,
          )
          .join(', ');
  }
  return { caption: actor ? `${actor} · ${description}` : description, positions: [...positions] };
}
export function frameEffects(game: Game, after: number) {
  return (game.board_history?.frames ?? [])
    .filter((f) => f.cursor > after)
    .flatMap((f) =>
      (f.effects ?? []).map((effect, i) => ({
        ...effect,
        key: `${game.board_history!.id}:${f.cursor}:${i}`,
      })),
    );
}

function lastIndex<T>(items: T[], match: (item: T) => boolean) {
  for (let i = items.length - 1; i >= 0; i--) if (match(items[i])) return i;
  return -1;
}
