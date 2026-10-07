/**
 * Local move prediction from the seat's player-filtered state.
 *
 * The browser only has a `stripSecret`ed game: hidden piles are in an
 * arbitrary order, the random stream is reset, unexplored tiles are blank,
 * opponents' hands and settings are masked and undo patches are removed.
 * A move is predicted only when its visible result does not depend on any of
 * that missing information. The server's state always remains authoritative.
 */
export interface PredictionEngine {
  tryMove(state: string, move: string, player: number): string;
  stripSecret(state: string, player?: number): string;
}

const HIDDEN_PILES = ['incidents_left', 'wonders_left', 'action_cards_left', 'objective_cards_left'] as const;
// Any concrete block works: exploring it must produce a different visible map.
const SAMPLE_BLOCK = ['Fertile', 'Forest', 'Mountain', 'Barren'];
// A different, valid u128 random state.
const SAMPLE_RNG = '170141183460469231731687303715884105727';

type Json = Record<string, any>;

/** Comparable public content of a player-filtered state. */
export function fingerprint(raw: string): string {
  const game = JSON.parse(raw) as Json;
  // Hidden pile order is reshuffled by every stripSecret; only sizes are public.
  for (const pile of HIDDEN_PILES) if (Array.isArray(game[pile])) game[pile] = game[pile].length;
  delete game.rng;
  delete game.seed;
  delete game.messages;
  return JSON.stringify(Object.fromEntries(Object.entries(game).sort(([a], [b]) => (a < b ? -1 : 1))));
}

/** The same public state with every kind of masked information resampled. */
export function resampleHidden(raw: string, seat: number): string {
  const game = JSON.parse(raw) as Json;
  for (const pile of HIDDEN_PILES) if (Array.isArray(game[pile])) game[pile] = [...game[pile]].reverse();
  game.rng = SAMPLE_RNG;
  for (const block of game.map?.unexplored_blocks ?? []) block.block = { terrain: SAMPLE_BLOCK };
  for (const player of game.players ?? []) if (player.id !== seat) player.settings = { skipRazeCity: true };
  return JSON.stringify(game);
}

/**
 * Moves whose outcome is decided by other players' secret choices, or that
 * reach rules evaluating masked cards, which the filtered state cannot even
 * run: combat (dice and opponents' tactics cards) and the status phase
 * (every player's objectives).
 */
function excluded(raw: string, move: unknown, seat: number) {
  const game = JSON.parse(raw) as Json;
  // The first recorded move names the playback history after the secret seed.
  if (!game.board_history?.id) return true;
  if (JSON.stringify(game.events ?? []).includes('"StatusPhase"')) return true;
  if (typeof move !== 'object' || move === null) return false;
  // Ending a turn in the last round may start the status phase.
  if ((move as Json).Playing === 'EndTurn' && game.round >= 3) return true;
  if ('ChooseCivilization' in move) return true;
  const destination = (move as Json).Movement?.Move?.destination;
  if (!destination) return false;
  // Capturing undefended settlers or cities is deterministic and stays eligible.
  return (game.players ?? []).some(
    (player: Json) =>
      player.id !== seat &&
      player.units?.some((unit: Json) => unit.position === destination && unit.unit_type !== 'Settler'),
  );
}

export type Prediction = { raw: string } | { reason: 'hidden' | 'rejected' | 'unsupported' };

/**
 * Executes `move` twice: on the received state and on a copy whose hidden
 * piles, random stream, unexplored tiles and opponent settings differ. Only an
 * identical visible result is a prediction. Returns the predicted state in the
 * same filtered form the server sends.
 *
 * Engine traps (`WebAssembly.RuntimeError`) are rethrown: the caller must stop
 * predicting, because a trapped module should not be exercised further.
 */
export function predictMove(engine: PredictionEngine, raw: string, move: unknown, seat: number): Prediction {
  if (excluded(raw, move, seat)) return { reason: 'unsupported' };
  const serialized = JSON.stringify(move);
  let predicted: string;
  try {
    predicted = engine.stripSecret(engine.tryMove(raw, serialized, seat), seat);
  } catch (error) {
    if (isTrap(error)) throw error;
    return { reason: 'rejected' };
  }
  try {
    const sample = engine.stripSecret(engine.tryMove(resampleHidden(raw, seat), serialized, seat), seat);
    if (fingerprint(sample) !== fingerprint(predicted)) return { reason: 'hidden' };
  } catch (error) {
    if (isTrap(error)) throw error;
    return { reason: 'hidden' };
  }
  return { raw: predicted };
}

export function isTrap(error: unknown) {
  return typeof WebAssembly !== 'undefined' && error instanceof WebAssembly.RuntimeError;
}

const lastTurn = (game: Json) => game.log?.at(-1)?.rounds?.at(-1)?.turns?.at(-1);

/**
 * Undo restores a previously received state, but the engine keeps the undone
 * action (without its results) in the log so it can be redone.
 */
export function predictUndo(previous: string, current: string): string | null {
  const before = JSON.parse(previous) as Json,
    after = JSON.parse(current) as Json;
  if (before.log_index !== after.log_index - 1) return null;
  const log = structuredClone(after.log);
  const actions = lastTurn({ log })?.actions;
  const undone = actions?.[after.log_index - 1];
  if (!undone) return null;
  delete undone.items;
  delete undone.log;
  delete undone.combat_stats;
  delete undone.undo;
  before.log = log;
  // Playback history is outside the undo patch: Undo only trims later frames.
  if (after.board_history) {
    const cursor = logLength(after) - 1;
    before.board_history = {
      ...after.board_history,
      frames: after.board_history.frames.filter((frame: Json) => frame.cursor <= cursor),
    };
  } else delete before.board_history;
  return JSON.stringify(before);
}

/** The engine's linear action log length, which is also the playback cursor. */
function logLength(game: Json) {
  const turns = (game.log ?? []).flatMap((age: Json) => age.rounds.flatMap((round: Json) => round.turns));
  return turns.reduce(
    (sum: number, turn: Json, i: number) =>
      sum + (i + 1 === turns.length ? Math.min(game.log_index, turn.actions.length) : turn.actions.length),
    0,
  );
}
