import { activeHistory } from './active-history.ts';
import type { BoardFrame, Game, LoggedAction } from './types.ts';

export const publicActions = (game: Game) =>
  activeHistory(game).flatMap((a) => a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])));

export function playingAction(action: LoggedAction): Record<string, unknown> | null {
  const move = action.action;
  const playing = move && typeof move === 'object' ? move.Playing : undefined;
  return playing && typeof playing === 'object' ? (playing as Record<string, unknown>) : null;
}

function boundary(action: LoggedAction, frame: BoardFrame): boolean {
  return (
    !!frame.combat ||
    !!action.combat_stats ||
    !!action.items?.some(
      (item) =>
        item.CombatRound ||
        item.CombatRoll ||
        item.Text?.startsWith('triggers the event ') ||
        (item.HandCard?.to &&
          typeof item.HandCard.to === 'object' &&
          'CompleteObjective' in item.HandCard.to),
    ) ||
    !!action.log?.some((line) => /^A new game event has been triggered:|^Combat round/.test(line))
  );
}

/** Payment and same-player decisions resolve their initiating action, not extra turns. */
export function groupPlaybackFrames(game: Game): BoardFrame[] {
  const frames = game.board_history?.frames ?? [];
  const actions = publicActions(game);
  const result: BoardFrame[] = [];
  let group: { player: number | null; title: string; exploration: boolean } | null = null;
  for (const [index, frame] of frames.entries()) {
    const interval = actions.slice(frames[index - 1]?.cursor ?? frame.cursor, frame.cursor);
    const action = interval.length === 1 ? interval[0] : undefined;
    const separate = !action || boundary(action, frame) || frame.ended_turn;
    const response = action?.action && typeof action.action === 'object' && 'Response' in action.action;
    const explorationResponse =
      response &&
      ('ExploreResolution' in (action!.action as { Response: Record<string, unknown> }).Response ||
        frame.title === 'Finish ship exploration');
    if (
      !separate &&
      response &&
      group &&
      (!group.exploration || explorationResponse) &&
      frame.actor === group.player &&
      result.length > 1
    ) {
      const previous = result.pop()!;
      result.push({
        ...frame,
        title: group.exploration ? 'Explore' : group.title,
        effects: [previous, frame].flatMap((f) =>
          (f.effects ?? []).map((e, i) => ({
            ...e,
            cursor: e.cursor ?? f.cursor,
            key: e.key ?? `${game.board_history!.id}:${f.cursor}:${i}`,
          })),
        ),
      });
      continue;
    }
    result.push(frame);
    const move = action?.action && typeof action.action === 'object' ? action.action.Movement : null;
    const exploration = !!move && typeof move === 'object' && 'Move' in move;
    group =
      !separate && action && (playingAction(action) || exploration)
        ? { player: frame.actor, title: frame.title, exploration }
        : null;
  }
  return result;
}
