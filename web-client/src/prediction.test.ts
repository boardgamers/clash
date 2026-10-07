import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { build } from 'esbuild';
import type { Controller as ControllerType } from './controller';
import type { Session } from './types';
import { fingerprint, predictMove, predictUndo } from './prediction.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
// Bundle the actual controller with the same engine's Node bridge.
const bundled = await build({
  entryPoints: [fileURLToPath(new URL('./controller.ts', import.meta.url))],
  bundle: true,
  write: false,
  platform: 'node',
  format: 'esm',
  logLevel: 'silent',
  plugins: [
    {
      name: 'node-engine',
      setup(plugin) {
        plugin.onResolve({ filter: /^\.\/bridge$/ }, () => ({ path: 'bridge', namespace: 'test-engine' }));
        plugin.onLoad({ filter: /.*/, namespace: 'test-engine' }, () => ({
          contents: `import {createRequire} from 'node:module'; const engine=createRequire(${JSON.stringify(import.meta.url)})('../.engine/server.js'); export async function loadBridge(){return engine;}`,
        }));
      },
    },
  ],
});
const { Controller } = (await import(
  `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString('base64')}`
)) as { Controller: typeof ControllerType };

const npcs = JSON.parse(await engine.init(2, [], {}, 'prediction-fixtures', {})).players.slice(2);
const move = (state: string, action: unknown, player = 0): string =>
  engine.tryMove(state, JSON.stringify(action), player);
const strip = (state: string, player = 0): string => engine.stripSecret(state, player);

/**
 * A saved test game with the public playback history a live game already has:
 * the position before `seed` as the engine records it.
 */
function fixture(name: string, seed: unknown, seat = 0) {
  const game = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p: { civilization: string }) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  const frames = JSON.parse(move(JSON.stringify(game), seed, seat)).board_history.frames;
  return JSON.stringify({ ...game, board_history: { id: `prediction-${name}`, frames: frames.slice(0, 1) } });
}

const collect = (position: string, pile: Record<string, number>) => ({
  Playing: {
    Collect: { city_position: 'C2', collections: [{ position, pile, times: 1 }], action_type: 'Collect' },
  },
});
const ore = collect('B1', { ore: 1 }),
  wood = collect('C2', { wood: 1 }),
  exploreC7 = { Movement: { Move: { units: [0], destination: 'C7' } } };

function harness() {
  const sent: string[] = [];
  const sounds: string[] = [];
  const logs: string[][] = [];
  let accept = true;
  let session!: Session;
  const controller = new Controller(
    {
      move: (m: string) => (sent.push(m), accept),
      replaceLog: (lines: string[]) => logs.push(lines),
      fetchState: () => true,
      setReplayInfo: () => true,
      updateSetting: () => true,
      updatePreference: () => true,
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
  );
  controller.audio.play = ((name: string) => sounds.push(name)) as typeof controller.audio.play;
  const off = controller.session.subscribe((s) => (session = s));
  return {
    controller,
    sent,
    sounds,
    logs,
    session: () => session,
    reject: () => (accept = false),
    close: () => {
      off();
      controller.destroy();
    },
  };
}

test('deterministic moves are predicted exactly, including Undo and Redo', () => {
  const raw = fixture('advances/collect_free_economy', ore);
  const predicted = predictMove(engine, strip(raw), ore, 0);
  assert.ok('raw' in predicted);
  const after = move(raw, ore);
  assert.equal(fingerprint(predicted.raw), fingerprint(strip(after)));
  const undone = move(after, 'Undo');
  assert.equal(fingerprint(predictUndo(strip(raw), strip(after))!), fingerprint(strip(undone)));
  const redo = predictMove(engine, strip(undone), 'Redo', 0);
  assert.ok('raw' in redo);
  assert.equal(fingerprint(redo.raw), fingerprint(strip(move(undone, 'Redo'))));
  const ended = predictMove(engine, strip(after), { Playing: 'EndTurn' }, 0);
  assert.ok('raw' in ended);
  assert.equal(fingerprint(ended.raw), fingerprint(strip(move(after, { Playing: 'EndTurn' }))));
});

test('moves revealing hidden tiles or cards, battles and setup choices are never predicted', () => {
  const explore = fixture('movement/explore_choose', exploreC7, 1);
  assert.deepEqual(predictMove(engine, strip(explore, 1), exploreC7, 1), { reason: 'hidden' });
  const research = { Playing: { Advance: { advance: 'Writing', payment: { food: 2 } } } };
  const seer = fixture('incidents/great_persons/great_seer', research);
  assert.deepEqual(predictMove(engine, strip(seer), research, 0), { reason: 'hidden' });
  // Combat is rejected before running the engine on masked tactics cards.
  const battle = JSON.parse(strip(explore, 1));
  const enemy = battle.players[0].units[0];
  enemy.unit_type = 'Infantry';
  assert.deepEqual(
    predictMove(
      engine,
      JSON.stringify(battle),
      { Movement: { Move: { units: [0], destination: enemy.position } } },
      1,
    ),
    { reason: 'unsupported' },
  );
  assert.deepEqual(predictMove(engine, strip(explore, 1), { ChooseCivilization: 'Rome' }, 1), {
    reason: 'unsupported',
  });
  // Without playback history the first recorded frame is named after the secret seed.
  const legacy = JSON.parse(explore);
  delete legacy.board_history;
  assert.deepEqual(predictMove(engine, strip(JSON.stringify(legacy), 1), { Playing: 'EndTurn' }, 1), {
    reason: 'unsupported',
  });
  // Illegal moves are left to the server's error message.
  assert.deepEqual(predictMove(engine, strip(explore, 1), collect('Z9', { ore: 1 }), 1), {
    reason: 'rejected',
  });
});

async function loaded() {
  const app = harness();
  const raw = fixture('advances/collect_free_economy', ore);
  app.controller.setPlayer(0);
  await app.controller.load(strip(raw));
  return { ...app, raw };
}

test('a predicted move shows at once and its confirmation changes nothing or replays nothing', async () => {
  const app = await loaded();
  const c = app.controller;
  try {
    const before = JSON.parse(app.raw).players[0].resources;
    c.submit(ore);
    const shown = app.session();
    assert.equal(shown.pending, false);
    assert.equal(shown.game!.players[0].resources!.ore, (before.ore ?? 0) + 1);
    assert.equal(shown.view!.canUndo, true);
    assert.deepEqual(app.sounds, ['collect']);
    assert.equal(shown.toast, 'Game updated.');
    assert.equal(app.logs.length, 2, 'the journal is published once for the prediction');
    // A resend of the earlier snapshot is not an acknowledgement.
    await c.load(strip(app.raw));
    const after = move(app.raw, ore);
    await c.load(strip(after));
    assert.equal(app.session(), shown, 'no re-render for the confirmed state');
    assert.deepEqual(app.sounds, ['collect']);
    assert.equal(app.logs.length, 2);
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [ore],
    );
    // Queries now use the confirmed state.
    assert.ok(c.researchReferenceView()!.canUndo);
  } finally {
    app.close();
  }
});

test('moves made before the server confirms are queued in order and sent one at a time', async () => {
  const app = await loaded();
  const c = app.controller;
  try {
    c.submit(ore);
    c.submit('Undo');
    assert.equal(app.session().view!.canRedo, true);
    c.submit(wood);
    assert.equal(app.session().pending, false);
    assert.equal(
      app.session().game!.players[0].resources!.wood,
      JSON.parse(app.raw).players[0].resources.wood + 1,
    );
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [ore],
      'later moves wait for the first result',
    );
    let raw = move(app.raw, ore);
    await c.load(strip(raw));
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [ore, 'Undo'],
    );
    raw = move(raw, 'Undo');
    await c.load(strip(raw));
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [ore, 'Undo', wood],
    );
    const shown = app.session();
    raw = move(raw, wood);
    await c.load(strip(raw));
    assert.equal(app.session(), shown);
    assert.equal(app.sent.length, 3);
    assert.deepEqual(app.sounds, ['collect', 'undo', 'collect']);
  } finally {
    app.close();
  }
});

test('a different server result replaces the prediction and drops moves that were not sent', async () => {
  const app = await loaded();
  const c = app.controller;
  try {
    c.submit(ore);
    c.submit({ Playing: 'EndTurn' });
    // Another tab collected wood instead.
    const actual = move(app.raw, wood);
    await c.load(strip(actual));
    assert.equal(app.session().pending, false);
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(actual).players[0].resources);
    assert.equal(app.session().toast, 'Game updated.');
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [ore],
    );
    await c.load(strip(move(actual, { Playing: 'EndTurn' })));
    assert.equal(app.sent.length, 1, 'the dropped End turn is never sent');
    assert.deepEqual(app.sounds, ['collect', 'confirm'], 'the server result plays nothing more');
  } finally {
    app.close();
  }
});

test('a rejected prediction rolls back to the server state and cancels queued moves', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = await loaded();
  const c = app.controller;
  try {
    const confirmed = app.session();
    c.submit(ore);
    c.submit({ Playing: 'EndTurn' });
    c.handleError('Not your turn');
    assert.deepEqual(app.session().game!.players[0].resources, confirmed.game!.players[0].resources);
    assert.equal(app.session().game!.actions_left, confirmed.game!.actions_left);
    assert.equal(app.session().error, 'Not your turn');
    assert.equal(app.session().pending, false);
    assert.equal(app.session().publicEffects?.length ?? 0, 0);
    t.mock.timers.tick(8000);
    assert.equal(app.session().error, 'Not your turn');
    assert.equal(app.sent.length, 1);
    // A failed send behaves the same way.
    app.reject();
    c.submit(ore);
    assert.match(app.session().error, /could not be sent/);
    assert.deepEqual(app.session().game!.players[0].resources, confirmed.game!.players[0].resources);
  } finally {
    app.close();
  }
});

test('an unpredictable move shows a pending state and applies the server result once', async () => {
  const app = harness();
  const c = app.controller;
  try {
    const raw = fixture('movement/explore_choose', exploreC7, 1);
    c.setPlayer(1);
    await c.load(strip(raw, 1));
    const shown = app.session();
    const explore = exploreC7;
    c.submit(explore);
    assert.equal(app.session().pending, true);
    assert.equal(app.session().game, shown.game, 'nothing is guessed about the unexplored tiles');
    assert.deepEqual(app.sounds, []);
    await c.load(strip(raw, 1));
    assert.equal(app.session().pending, true, 'a resend does not count as the result');
    const after = move(raw, explore, 1);
    await c.load(strip(after, 1));
    assert.equal(app.session().pending, false);
    assert.notEqual(app.session().game, shown.game);
    assert.equal(app.sounds.length, 1);
  } finally {
    app.close();
  }
});

test('a confirmation timeout withdraws the prediction and a late result applies without a second sound', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = await loaded();
  const c = app.controller;
  try {
    const confirmed = app.session().game!;
    c.submit(ore);
    c.submit({ Playing: 'EndTurn' });
    t.mock.timers.tick(8000);
    assert.deepEqual(app.session().game!.players[0].resources, confirmed.players[0].resources);
    assert.equal(app.session().pending, true);
    assert.match(app.session().error, /Waiting for the game/);
    await c.load(strip(move(app.raw, ore)));
    assert.equal(app.session().pending, false);
    assert.equal(app.session().error, '');
    assert.equal(
      app.session().game!.players[0].resources!.ore,
      (confirmed.players[0].resources!.ore ?? 0) + 1,
    );
    assert.deepEqual(app.sounds, ['collect', 'confirm']);
    assert.equal(app.sent.length, 1, 'the queued End turn was withdrawn');
  } finally {
    app.close();
  }
});

test('predictions can be disabled, as for tutorials', async () => {
  const app = harness();
  const c = new Controller(
    {
      move: (m: string) => (app.sent.push(m), true),
      replaceLog: () => true,
      fetchState: () => true,
      setReplayInfo: () => true,
      updateSetting: () => true,
      updatePreference: () => true,
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
    { predict: false },
  );
  try {
    c.setPlayer(0);
    await c.load(strip(fixture('advances/collect_free_economy', ore)));
    c.submit(ore);
    assert.equal(get(c).pending, true);
  } finally {
    c.destroy();
    app.close();
  }
});

function get(c: ControllerType) {
  let value!: Session;
  c.session.subscribe((s) => (value = s))();
  return value;
}
