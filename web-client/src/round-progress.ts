import type { Game, PlayerView } from './types.ts';

export type RoundStatus = 'done' | 'current' | 'upcoming' | 'left';
export interface PlayerTurn {
  player: PlayerView;
  order?: number;
  status?: RoundStatus;
}

export function roundProgress(game: Game | null, players: PlayerView[]): PlayerTurn[] {
  if (!game || game.state === 'Finished' || game.state === 'ChooseCivilization' || game.round < 1)
    return players.map((player) => ({ player }));
  const start = game.starting_player_index ?? 0;
  const ordered = [...players].sort(
    (a, b) => Number(a.index < start) - Number(b.index < start) || a.index - b.index,
  );
  const playing = ordered.filter((p) => !game.dropped_players?.includes(p.index));
  // The turn owner is distinct from activePlayer: an opponent may currently be
  // choosing casualties, placing a settler, or completing an objective.
  const current = playing.findIndex((p) => p.index === game.current_player_index);
  return [...playing, ...ordered.filter((p) => game.dropped_players?.includes(p.index))].map((player) => {
    const index = playing.indexOf(player);
    return {
      player,
      order: index < 0 ? undefined : index + 1,
      status:
        index < 0
          ? 'left'
          : game.round > 3
            ? 'done'
            : current < 0
              ? undefined
              : index < current
                ? 'done'
                : index === current
                  ? 'current'
                  : 'upcoming',
    };
  });
}

export function personalRoundLabel(round: number, status?: RoundStatus): string | null {
  if (round !== 3 || !status || status === 'left') return null;
  return status === 'done'
    ? 'Final turn complete'
    : status === 'current'
      ? 'Your final turn'
      : 'Final turn ahead';
}
