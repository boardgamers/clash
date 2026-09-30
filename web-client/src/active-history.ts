import type { Game } from './types.ts';

// The last turn can also contain undone commands retained for Redo.
export function activeHistory(game: Game): NonNullable<Game['log']> {
  const ages = game.log ?? [];
  return ages.map((age, a) =>
    a + 1 !== ages.length
      ? age
      : {
          ...age,
          rounds: age.rounds.map((round, r) =>
            r + 1 !== age.rounds.length
              ? round
              : {
                  ...round,
                  turns: round.turns.map((turn, t) =>
                    t + 1 !== round.turns.length
                      ? turn
                      : {
                          ...turn,
                          actions: turn.actions?.slice(0, game.log_index),
                        },
                  ),
                },
          ),
        },
  );
}
