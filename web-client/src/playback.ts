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
