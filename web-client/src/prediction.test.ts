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
  assert.deepEqual(predictMove(engine, JSON.stringify(legacy), { Playing: 'EndTurn' }, 1), {
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

test('slow confirmations keep predictions and queued moves without replaying their effects', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = await loaded();
  const c = app.controller;
  try {
    const confirmed = app.session().game!;
    c.submit(ore);
    c.submit({ Playing: 'EndTurn' });
    const shownGame = app.session().game;
    t.mock.timers.tick(8000);
    assert.equal(app.session().game, shownGame);
    assert.equal(app.session().pending, false);
    assert.equal(app.session().error, '');
    await c.load(strip(move(app.raw, ore)));
    assert.equal(app.session().pending, false);
    assert.equal(app.session().error, '');
    assert.equal(
      app.session().game!.players[0].resources!.ore,
      (confirmed.players[0].resources!.ore ?? 0) + 1,
    );
    assert.deepEqual(app.sounds, ['collect', 'confirm']);
    assert.equal(app.sent.length, 2, 'the queued End turn is sent after the first acknowledgement');
    await c.load(strip(move(move(app.raw, ore), { Playing: 'EndTurn' })));
    assert.equal(app.session().game, shownGame);
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

test('end-of-age pirate placements and undo predict and queue without waiting', async () => {
  const first = { Response: { SelectPositions: ['A2'] } };
  const second = { Response: { SelectPositions: ['D2'] } };
  const raw = fixture('incidents/pirates_spawn.outcome1', first);
  const app = harness();
  const c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(strip(raw));
    c.submit(first);
    assert.equal(app.session().pending, false, 'the next pirate choice opens immediately');
    const afterFirst = move(raw, first);
    assert.equal(fingerprint(JSON.stringify(app.session().game)), fingerprint(strip(afterFirst)));
    c.submit(second);
    assert.equal(app.session().pending, false, 'the second pirate placement also predicts');
    const afterSecond = move(afterFirst, second);
    assert.equal(app.session().view!.decision!.fields.length, 1, 'the incident reward is already available');
    c.submit('Undo');
    assert.equal(app.session().pending, false, 'undoing a pirate placement is immediate');
    const undone = move(afterSecond, 'Undo');
    assert.equal(fingerprint(JSON.stringify(app.session().game)), fingerprint(strip(undone)));
    assert.equal(app.sent.length, 1, 'only the first move is sent before acknowledgement');
    const visible = app.session();
    for (const confirmed of [afterFirst, afterSecond, undone]) await c.load(strip(confirmed));
    assert.equal(app.session(), visible, 'confirmations preserve the latest predicted choice');
    assert.deepEqual(
      app.sent.map((m) => JSON.parse(m)),
      [first, second, 'Undo'],
    );
    c.submit(second);
    await c.load(strip(afterSecond));
    const payment = { Response: { Payment: [{ ore: 1 }] } };
    assert.ok('raw' in predictMove(engine, strip(afterSecond), payment, 0));
    c.submit(payment);
    assert.equal(app.session().pending, false, 'the visible bribe also continues immediately');
  } finally {
    app.close();
  }
});

test('the last end-of-age free advance waits when it draws hidden cards', () => {
  const end = { Playing: 'EndTurn' };
  let raw = fixture('status_phase/free_advance', end);
  raw = move(raw, end);
  raw = move(raw, { Response: { SelectAdvance: 'Storage' } }, 1);
  assert.deepEqual(predictMove(engine, strip(raw), { Response: { SelectAdvance: 'Philosophy' } }, 0), {
    reason: 'hidden',
  });
});

test('a fresh game exposes a stable public history ID and predicts its first action and Undo', async () => {
  const raw = await engine.init(2, [], { civilization: 'Random' }, 'first-action-prediction', {});
  const seat = engine.currentPlayer(raw);
  const publicRaw = strip(raw, seat),
    initial = JSON.parse(publicRaw);
  assert.ok(initial.board_history.id);
  assert.deepEqual(initial.board_history.frames, []);
  assert.ok(!initial.seed);
  const app = harness();
  try {
    app.controller.setPlayer(seat);
    await app.controller.load(publicRaw);
    const city = app.session().view!.cities.find((c) => !c.reason && c.choices.length)!;
    const choice = city.choices[0];
    const preview = JSON.parse(
      engine.webCollectPreview(publicRaw, seat, city.position, JSON.stringify([{ ...choice, times: 1 }])),
    );
    app.controller.submit(preview.action);
    assert.equal(app.session().pending, false);
    const after = move(raw, preview.action, seat);
    assert.equal(JSON.parse(after).board_history.id, initial.board_history.id);
    app.controller.submit('Undo');
    assert.equal(app.session().pending, false);
    assert.deepEqual(app.session().game!.players[seat].resources, initial.players[seat].resources);
    await app.controller.load(strip(after, seat));
    const shown = app.session();
    await app.controller.load(strip(move(after, 'Undo', seat), seat));
    assert.equal(app.session(), shown);
  } finally {
    app.close();
  }
});

test('normal actions use prediction by default across construction, recruitment, happiness, research and abilities', async () => {
  const g = JSON.parse(await engine.init(2, [], { civilization: 'Random' }, 'all-action-prediction', {}));
  const seat = engine.currentPlayer(JSON.stringify(g)),
    p = g.players[seat];
  p.civilization = 'Rome';
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  p.advances = ['Farming', 'Mining', 'Tactics', 'Myths', 'Fishing', 'Bartering', 'Storage', 'Taxes'];
  p.incident_tokens = 3;
  p.cities[0].mood_state = 'Neutral';
  const empty = g.map.tiles.find(
    ([pos, terrain]: string[]) =>
      ['Fertile', 'Forest', 'Mountain'].includes(terrain) &&
      !g.players.some((q: any) => (q.cities ?? []).some((c: any) => c.position === pos)),
  );
  p.cities.push({ position: empty[0], mood_state: 'Happy' });
  const raw = JSON.stringify(g),
    filtered = strip(raw, seat);
  const view = JSON.parse(engine.webView(filtered, seat));
  const city = view.cityActions[0];
  const query = (q: unknown) => JSON.parse(engine.webQuery(filtered, seat, JSON.stringify(q)));
  const actions: [string, unknown][] = [
    ['construction', city.buildings.find((b: any) => b.name === 'Fortress').choices[0].action],
    [
      'recruitment',
      query({ kind: 'recruit', city: city.position, units: { infantry: 1 }, replaced: [] }).action,
    ],
    ['happiness', city.happiness[0].action],
    ['research', view.advances.find((a: any) => a.id === 'Irrigation' || a.name === 'Irrigation').action],
    ['ability', view.specialActions.find((a: any) => a.name === 'Taxes').action],
    ['end turn', { Playing: 'EndTurn' }],
  ];
  for (const [name, action] of actions) {
    assert.ok(action, name);
    const prediction = predictMove(engine, filtered, action, seat);
    assert.ok('raw' in prediction, `${name}: ${JSON.stringify(prediction)}`);
    assert.equal(fingerprint(prediction.raw), fingerprint(strip(move(raw, action, seat), seat)), name);
  }
});

test('round three End turn predicts until it actually reaches objective completion', () => {
  const g = JSON.parse(fixture('advances/collect_free_economy', ore));
  g.round = 3;
  g.current_player_index = 0;
  g.starting_player_index = 0;
  const raw = JSON.stringify(g),
    end = { Playing: 'EndTurn' };
  const prediction = predictMove(engine, strip(raw), end, 0);
  assert.ok('raw' in prediction);
  const next = move(raw, end);
  assert.equal(fingerprint(prediction.raw), fingerprint(strip(next)));
  assert.deepEqual(predictMove(engine, strip(next, 1), end, 1), { reason: 'unsupported' });
});

test('rolls and hidden draws wait even when sampled visible outcomes could coincide', () => {
  const before = { board_history: { id: 'known', frames: [] }, rng: '1', action_cards_left: [1, 2] };
  for (const change of [{ rng: '2' }, { dice_roll_log: [1] }, { action_cards_left: [2] }]) {
    let executions = 0;
    const simulation = {
      tryMove: () => {
        executions++;
        return JSON.stringify({ ...before, ...change });
      },
      stripSecret: () => JSON.stringify(before),
    };
    assert.deepEqual(predictMove(simulation, JSON.stringify(before), { Playing: 'EndTurn' }, 0), {
      reason: 'hidden',
    });
    assert.equal(executions, 1);
  }
});

test('Spy waits before looking at a masked hand without trapping the local engine', () => {
  const game = JSON.parse(
    readFileSync(new URL('../../server/tests/test_games/action_cards/spy.json', import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p: any) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  const raw = JSON.stringify(game);
  const card = JSON.parse(engine.webView(strip(raw), 0)).actionCards.find((c: any) => c.id === 7);
  const payment = move(raw, card.action);
  assert.deepEqual(
    predictMove(engine, strip(payment), { Response: { Payment: [{ culture_tokens: 1 }] } }, 0),
    { reason: 'unsupported' },
  );
  // The reveal boundary must not disable prediction for subsequent ordinary actions.
  const next = fixture('advances/collect_free_economy', ore);
  assert.ok('raw' in predictMove(engine, strip(next), ore, 0));
});

test('prediction fingerprints ignore nested map field order but preserve values and array order', () => {
  const a = {
    players: [{ id: 0, resources: { food: 2, gold: 1 }, event_info: { first: 'a', second: 'b' } }],
    events: [{ details: { target: 'A1', amount: 1 } }],
  };
  const b = {
    events: [{ details: { amount: 1, target: 'A1' } }],
    players: [{ event_info: { second: 'b', first: 'a' }, resources: { gold: 1, food: 2 }, id: 0 }],
  };
  assert.equal(fingerprint(JSON.stringify(a)), fingerprint(JSON.stringify(b)));
  b.players[0].resources.food = 3;
  assert.notEqual(fingerprint(JSON.stringify(a)), fingerprint(JSON.stringify(b)));
  assert.notEqual(fingerprint('{"choices":[1,2]}'), fingerprint('{"choices":[2,1]}'));
});

test('Theaters opens its direction choice and completes locally before any server acknowledgement', async () => {
  const g = JSON.parse(fixture('advances/sanitation_and_draft', { Playing: 'EndTurn' }));
  g.players[0].advances.push('Arts', 'Theaters');
  g.players[0].resources.mood_tokens = 2;
  g.players[0].resources.culture_tokens = 2;
  // Multiple nested map entries exercise hash-map iteration in engine serialization.
  g.players[0].event_info = { first: 'a', second: 'b', third: 'c' };
  const raw = JSON.stringify(g),
    app = harness();
  try {
    app.controller.setPlayer(0);
    await app.controller.load(strip(raw));
    const theater = app.session().view!.specialActions!.find((a) => a.name === 'Theaters')!;
    app.controller.submit(theater.action);
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().pending, false);
    assert.equal(app.session().view!.decision?.name, 'Theaters');
    const response = app.controller.query<{ action: any }>({
      kind: 'decision',
      values: [],
      payments: [{ mood_tokens: 1 }],
    }).action;
    app.controller.submit(response);
    assert.equal(app.sent.length, 1, 'Second move is queued without waiting to update the UI');
    assert.equal(app.session().pending, false);
    assert.equal(app.session().game!.players[0].resources!.culture_tokens, 3);
    const first = move(raw, theater.action);
    await app.controller.load(strip(first));
    assert.equal(app.sent.length, 2);
    await app.controller.load(strip(move(first, response)));
    assert.equal(app.session().pending, false);
    assert.equal(app.session().game!.players[0].resources!.culture_tokens, 3);
  } finally {
    app.close();
  }
});

test('Undo counts empty historic turns as zero actions', () => {
  const previous = { log_index: 1, log: [{ rounds: [{ turns: [{}, { actions: [{ action: 'a' }] }] }] }] };
  const current = structuredClone(previous);
  current.log_index = 2;
  current.log[0].rounds[0].turns[1].actions!.push({ action: 'b' });
  const board_history = { id: 'empty-turn', frames: [{ cursor: 0 }, { cursor: 1 }, { cursor: 2 }] };
  const undone = JSON.parse(
    predictUndo(JSON.stringify(previous), JSON.stringify({ ...current, board_history }))!,
  );
  assert.equal(undone.log_index, 1);
  assert.deepEqual(undone.board_history.frames, [{ cursor: 0 }, { cursor: 1 }]);
});
