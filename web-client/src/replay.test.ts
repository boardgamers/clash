import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const engine = createRequire(import.meta.url)('../.engine/server.js');

const legacyHistory = (game: any) => ({
  ...game,
  action_log: game.log.map((age: any) => ({
    rounds: age.rounds.map((round: any) => ({
      players: round.turns.map((turn: any) => ({
        index: turn.actions?.[0]?.player ?? 0,
        actions: (turn.actions ?? []).map((item: any) => ({ action: item.action })),
      })),
    })),
  })),
});

test('replay rejects current saves instead of silently replacing them with setup', async () => {
  const game = await engine.init(3, [], { civilization: 'DraftThree' }, 'replay-safety', {});
  for (const to of [60, 62]) {
    assert.throws(
      () => engine.replay(game, { to }),
      /Cannot replay this save: supported action_log history is required/,
    );
  }
});

test('replay rejects empty legacy histories and targets beyond their end', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'legacy-replay', {}));
  assert.throws(
    () => engine.replay(JSON.stringify({ ...game, action_log: [] }), {}),
    /Cannot replay an empty action history/,
  );
  const legacy = JSON.stringify(legacyHistory(game));
  assert.throws(() => engine.replay(legacy, { to: 60 }), /outside the action history/);
  assert.throws(() => engine.replay(legacy, { to: 'invalid' }), /Invalid replay options/);
});

test('supported legacy replay still reconstructs recorded moves and rejects illegal ones', async () => {
  let game = await engine.init(2, [], {}, 'legacy-replay', {});
  const player = engine.currentPlayer(game);
  game = engine.tryMove(
    game,
    JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } }),
    player,
  );
  const saved = JSON.parse(game);
  const legacy = legacyHistory(saved);
  const replayed = engine.replay(JSON.stringify(legacy), {});
  assert.equal(engine.logLength(replayed), engine.logLength(game));
  assert.deepEqual(JSON.parse(replayed).players, saved.players);
  legacy.action_log
    .at(-1)
    .rounds.at(-1)
    .players.at(-1)
    .actions.push({
      action: { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } },
    });
  assert.throws(() => engine.replay(JSON.stringify(legacy), {}), /Failed to execute action/);
});
