import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (game: string, action: unknown, player = engine.currentPlayer(game)) =>
  engine.tryMove(game, JSON.stringify(action), player);
const storage = { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } };
const compareState = (actual: string, expected: string) => {
  const a = JSON.parse(actual),
    b = JSON.parse(expected);
  for (const key of [
    'state',
    'players',
    'map',
    'age',
    'round',
    'current_player_index',
    'starting_player_index',
    'actions_left',
    'rng',
    'action_cards_left',
    'objective_cards_left',
    'wonders_left',
    'incidents_left',
    'events',
  ])
    assert.deepEqual(a[key], b[key], key);
  assert.equal(engine.logLength(actual), engine.logLength(expected));
};

for (const civilization of ['Random', 'ChooseCivilization', 'DraftThree']) {
  test(`current ${civilization} history reconstructs setup and exact move-count targets`, async () => {
    let game = await engine.init(3, [], { civilization }, 'current-replay', {});
    if (civilization === 'ChooseCivilization') {
      for (const name of ['Babylonia', 'Rome', 'Phoenicia']) game = move(game, { ChooseCivilization: name });
    } else if (civilization === 'DraftThree') {
      const offers = JSON.parse(game).civilization_draft.offers;
      for (const i of [2, 0, 1]) game = move(game, { ChooseCivilization: offers[i][1] }, i);
    }
    const setup = game;
    game = move(game, storage);
    const advanced = game;
    game = move(game, { Playing: 'EndTurn' });
    compareState(engine.replay(game, { to: engine.logLength(setup) }), setup);
    compareState(engine.replay(game, { to: engine.logLength(advanced) }), advanced);
    compareState(engine.replay(game, { to: engine.logLength(game) }), game);
    compareState(engine.replay(game, {}), game);
  });
}

test('current replay follows the active undo cursor instead of replaying undone moves', async () => {
  const setup = await engine.init(2, [], {}, 'undo-replay', {});
  const advanced = move(setup, storage);
  const undone = move(advanced, 'Undo');
  compareState(engine.replay(undone, {}), setup);
});

test('replay rejects absent or empty current history and invalid targets and cursors', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'invalid-replay', {}));
  const { log, ...withoutHistory } = game;
  assert.throws(() => engine.replay(JSON.stringify(withoutHistory), {}), /current log history is required/);
  assert.throws(() => engine.replay(JSON.stringify({ ...game, log: [] }), {}), /empty action history/);
  assert.throws(() => engine.replay(JSON.stringify(game), { to: 60 }), /outside the action history/);
  assert.throws(() => engine.replay(JSON.stringify(game), { to: 'invalid' }), /Invalid replay options/);
  assert.throws(() => engine.replay(JSON.stringify(game), { to: 0 }), /complete action boundary/);
  assert.throws(
    () => engine.replay(JSON.stringify({ ...game, log_index: 100 }), {}),
    /Invalid replay history cursor/,
  );
});

test('illegal recorded actions throw instead of returning a partial replay', async () => {
  const setup = await engine.init(2, [], {}, 'illegal-replay', {});
  const saved = JSON.parse(move(setup, storage));
  saved.log.at(-1).rounds.at(-1).turns.at(-1).actions.at(-1).action = { Response: { Bool: true } };
  assert.throws(() => engine.replay(JSON.stringify(saved), {}), /Failed to replay move/);
});
