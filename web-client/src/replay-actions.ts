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

export function frameHasContent(game: Game, before: BoardFrame | undefined, frame: BoardFrame): boolean {
  if (frame.effects?.length) return true;
  const actions = publicActions(game).slice(before?.cursor ?? frame.cursor, frame.cursor);
  if (
    actions.some((action) =>
      action.items?.some(
        (item) =>
          Object.keys(item).some(
            (key) =>
              !['player', 'origin', 'Text'].includes(key) &&
              (key !== 'Resources' || Object.values(item.Resources?.resources ?? {}).some(Boolean)),
          ) || item.Text?.startsWith('triggers the event '),
      ),
    )
  )
    return true;
  if (actions.some((action) => action.combat_stats)) return true;
  if (frame.ended_turn || frame.title === 'End turn') return false;
  if (
    before &&
    JSON.stringify([frame.tiles, frame.players], (key, value) =>
      ['activations', 'angry_activation', 'movement_restrictions'].includes(key) ? undefined : value,
    ) !==
      JSON.stringify([before.tiles, before.players], (key, value) =>
        ['activations', 'angry_activation', 'movement_restrictions'].includes(key) ? undefined : value,
      )
  )
    return true;
  if (actions.length)
    return actions.some((action) => {
      const move = action.action;
      if (!move || typeof move !== 'object') return false;
      if (move.Playing && move.Playing !== 'EndTurn') return true;
      if (move.Movement && move.Movement !== 'Stop') return true;
      if ('ChooseCivilization' in move) return true;
      const response = move.Response;
      return (
        !!response &&
        typeof response === 'object' &&
        'SelectPositions' in response &&
        Array.isArray(response.SelectPositions) &&
        response.SelectPositions.length > 0
      );
    });
  return !['End turn', 'Earlier position', 'Raze city', 'Resolve choice'].includes(frame.title);
}

export function playbackSteps(game: Game, start: number, end: number): number[] {
  const frames = game.board_history?.frames ?? [];
  return [
    start,
    ...frames.flatMap((frame, index) =>
      index > start && index <= end && frameHasContent(game, frames[index - 1], frame) ? [index] : [],
    ),
  ];
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
    const previousFrame = frames[index - 1];
    if (action && previousFrame && !frame.ended_turn && !frameHasContent(game, previousFrame, frame)) {
      continue;
    }
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
