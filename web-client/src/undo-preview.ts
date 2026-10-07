import type { Game, View } from './types.ts';

interface Snapshot {
  game: Game;
  raw: string;
}

const turn = (game: Game) => game.log?.at(-1)?.rounds.at(-1)?.turns.at(-1);
const prefix = (game: Game, count: number) =>
  JSON.stringify((turn(game)?.actions ?? []).slice(0, count).map((entry) => entry.action));

/** Only retain states shown for this seat, never simulate hidden undo patches. */
export class UndoPreview {
  private scope = '';
  private snapshots: Snapshot[] = [];

  clear() {
    this.scope = '';
    this.snapshots = [];
  }

  remember(raw: string, game: Game, view: View, seat: number | undefined) {
    if (seat === undefined || !game.board_history?.id || !turn(game)) {
      this.clear();
      return;
    }
    const scope = JSON.stringify([
      game.board_history.id,
      seat,
      game.current_player_index,
      game.age,
      game.round,
      game.log?.length,
      game.log?.at(-1)?.rounds.length,
      game.log?.at(-1)?.rounds.at(-1)?.turns.length,
      turn(game)?.turn_type,
    ]);
    if (scope !== this.scope || !view.canUndo) this.snapshots = [];
    this.scope = scope;
    // An undo or a different branch must discard superseded states.
    this.snapshots = this.snapshots.filter(
      (snapshot) =>
        snapshot.game.log_index < game.log_index &&
        prefix(snapshot.game, snapshot.game.log_index) === prefix(game, snapshot.game.log_index),
    );
    this.snapshots.push({ game, raw });
    while (
      this.snapshots.length > 8 ||
      this.snapshots.reduce((sum, snapshot) => sum + snapshot.raw.length * 2, 0) > 8 * 1024 * 1024
    )
      this.snapshots.shift();
  }

  /** The serialized state one undo step before `game`, if it was shown on this branch. */
  previous(game: Game, view: View) {
    if (!view.canUndo) return undefined;
    return this.snapshots.find(
      (snapshot) =>
        snapshot.game.log_index === game.log_index - 1 &&
        prefix(snapshot.game, snapshot.game.log_index) === prefix(game, snapshot.game.log_index),
    )?.raw;
  }
}
