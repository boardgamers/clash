import { frameDetails } from './playback.ts';
import { frameHasContent, publicActions } from './replay-actions.ts';
import type { Game, View } from './types.ts';

export function liveActionMessages(before: Game, game: Game, seat?: number, view?: View | null): string[] {
  if (before.board_history?.id !== game.board_history?.id) return [];
  const cursor = publicActions(before).length;
  if (publicActions(game).length <= cursor) return [];
  const frames = game.board_history?.frames ?? [];
  return frames.flatMap((frame, index) => {
    if (frame.cursor <= cursor || frame.actor === seat) return [];
    const previous = frames[index - 1] ?? before.board_history?.frames.at(-1);
    if (!previous) return [];
    const start = { ...previous, cursor: Math.max(previous.cursor, cursor) };
    if (!frameHasContent(game, start, frame)) return [];
    return [frameDetails(start, frame, game, view?.advances, view?.cardCatalog).caption];
  });
}
