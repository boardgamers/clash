import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { build } from 'esbuild';
import type { Controller as ControllerType } from './controller';
import type { Session } from './types';
import { contextualCards, activeCollectionCard } from './contextual-cards.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
// Bundle the actual controller so Node can run its browser TypeScript imports.
// Only the WASM loader changes: the tests use the same engine's Node bridge.
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

function paymentController() {
  const sent: string[] = [];
  const preferences: { name: string; value: unknown }[] = [];
  let accept = true;
  let session: Session;
  const controller = new Controller(
    {
      move: (move: string) => {
        sent.push(move);
        return accept;
      },
      replaceLog: () => {},
      fetchState: () => {},
      setReplayInfo: () => {},
      updatePreference: (name: string, value: unknown) => {
        preferences.push({ name, value });
        return true;
      },
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
  );
  const off = controller.session.subscribe((s) => {
    session = s;
  });
  return {
    controller,
    sent,
    preferences,
    session: () => session,
    reject: () => {
      accept = false;
    },
    close: () => {
      off();
      controller.destroy();
    },
  };
}
const npcs = JSON.parse(await engine.init(2, [], {}, 'payment-fixtures', {})).players.slice(2);
const fixture = (name: string) => {
  const game = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p: { civilization: string }) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return JSON.stringify(game);
};

test('Mass Production from Collect adds two tiles and keeps the chosen city, tiles and Free Economy', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/collect_free_economy'));
    game.players[0].action_cards = [29, 19];
    let raw = JSON.stringify(game);
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    c.beginCollect();
    const economy = app.session().view!.collectActions!.find((a) => a.name === 'Free Economy')!;
    c.switchCollectVariant(economy.value);
    const city = app.session().view!.cities.find((city) => city.position === app.session().city)!;
    c.toggleChoice(city.choices[0]);
    const selection = structuredClone(app.session().selection);
    assert.ok(contextualCards(app.session().view, 'collect').some((o) => o.card.name === 'Mass Production'));
    c.playContextualCard(29, 'collect');
    c.playContextualCard(29, 'collect');
    assert.equal(app.sent.length, 1, 'a card cannot be sent twice');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    assert.deepEqual(app.session().selection, selection, 'unchanged snapshots keep the local choices');
    raw = engine.tryMove(raw, app.sent[0], 0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().mode, 'collect');
    assert.equal(app.session().city, city.position);
    assert.deepEqual(app.session().selection, selection);
    assert.deepEqual(app.session().collectVariant, economy.value);
    assert.ok(app.session().preview?.action);
    assert.equal(
      app.session().view!.cities.find((c) => c.position === city.position)!.capacity,
      city.capacity + 2,
    );
    assert.equal(activeCollectionCard(app.session().game), 'Mass Production · +2 tiles');
    assert.deepEqual(
      contextualCards(app.session().view, 'collect'),
      [],
      'Production Focus cannot stack with Mass Production',
    );
    c.collect();
    raw = engine.tryMove(raw, app.sent[1], 0);
    await c.load(engine.stripSecret(raw, 0));
    // The displayed Free Economy fee may be a separate engine continuation.
    if (app.sent.length > 2) raw = engine.tryMove(raw, app.sent[2], 0);
    assert.equal(activeCollectionCard(JSON.parse(raw)), null, 'boost is consumed by collection');
  } finally {
    app.close();
  }
});

test('a rejected contextual card leaves the collection draft intact', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/collect_free_economy'));
    game.players[0].action_cards = [19];
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.beginCollect();
    c.toggleChoice(app.session().view!.cities[0].choices[0]);
    const selection = structuredClone(app.session().selection);
    app.reject();
    c.playContextualCard(19, 'collect');
    assert.equal(app.session().pending, false);
    assert.equal(app.session().mode, 'collect');
    assert.deepEqual(app.session().selection, selection);
    assert.ok(app.session().error);
  } finally {
    app.close();
  }
});

test('research card shortcut keeps the card’s advance choice instead of returning to normal research', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(await engine.init(2, [], { civilization: 'Random' }, 'contextual-research', {}));
    const seat = game.current_player_index;
    game.players[seat].action_cards = [1];
    game.players[seat].resources.culture_tokens = 2;
    let raw = JSON.stringify(game);
    c.setPlayer(seat);
    await c.load(engine.stripSecret(raw, seat));
    c.patch({ mode: 'research' });
    c.playContextualCard(1, 'research');
    raw = engine.tryMove(raw, app.sent[0], seat);
    await c.load(engine.stripSecret(raw, seat));
    if (app.sent.length > 1) {
      raw = engine.tryMove(raw, app.sent[1], seat);
      await c.load(engine.stripSecret(raw, seat));
    }
    assert.equal(app.session().mode, 'research');
    assert.equal(app.session().view!.decision!.advanceSelection, true);
    assert.equal(app.session().view!.decision!.advanceMode, 'free');
    assert.deepEqual(contextualCards(app.session().view, 'research'), []);
  } finally {
    app.close();
  }
});

test('Free Economy pays the displayed fee once, with no second click or duplicate snapshot submission', async () => {
  const app = paymentController(),
    c = app.controller;
  let state = fixture('advances/collect_free_economy');
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(state, 0));
    const fee = app.session().view!.collectActions!.find((a) => a.name === 'Free Economy')!;
    c.beginCollect();
    c.patch({ collectVariant: fee.value });
    c.toggleChoice(app.session().view!.cities[0].choices[0]);
    assert.ok(app.session().preview);
    const before = JSON.parse(state);
    c.collect();
    assert.equal(app.sent.length, 1);
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 1);
    state = engine.tryMove(state, app.sent[0], 0);
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 2);
    assert.equal(app.session().automaticPayment, true);
    assert.equal(app.session().pending, true);
    assert.deepEqual(JSON.parse(app.sent[1]), { Response: { Payment: [{ mood_tokens: 1 }] } });
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 2);
    state = engine.tryMove(state, app.sent[1], 0);
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 2);
    assert.equal(app.session().automaticPayment, false);
    assert.equal(app.session().pending, false);
    assert.equal(
      JSON.parse(state).players[0].resources.mood_tokens,
      before.players[0].resources.mood_tokens - 1,
    );
    assert.equal(JSON.parse(state).actions_left, before.actions_left);
  } finally {
    app.close();
  }
});

test('only an accepted matching quote continues payment, and a send failure restores the controls', async () => {
  for (const mode of ['unquoted', 'changed', 'reconnect', 'rejected']) {
    const app = paymentController(),
      c = app.controller;
    let state = fixture('action_cards/spy');
    try {
      c.setPlayer(0);
      await c.load(engine.stripSecret(state, 0));
      const card = app.session().view!.actionCards!.find((card) => card.id === 7)!;
      if (mode !== 'reconnect')
        c.submit(
          card.action!,
          mode === 'unquoted' ? undefined : mode === 'changed' ? { culture_tokens: 2 } : card.cost,
        );
      if (mode === 'rejected') app.reject();
      state = engine.tryMove(state, JSON.stringify(card.action), 0);
      await c.load(engine.stripSecret(state, 0));
      assert.equal(app.sent.length, mode === 'rejected' ? 2 : mode === 'reconnect' ? 0 : 1);
      assert.equal(app.session().automaticPayment, false);
      assert.equal(app.session().pending, false);
      assert.equal(app.session().view!.decision!.name, 'Pay for action');
      if (mode === 'rejected') assert.match(app.session().error, /could not be sent/);
    } finally {
      app.close();
    }
  }
});

test('Sun Tzu still offers mood or culture when both payments are possible', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'sun-tzu-payment', {});
    for (const civilization of ['China', 'Rome'])
      state = engine.tryMove(
        state,
        JSON.stringify({ ChooseCivilization: civilization }),
        engine.currentPlayer(state),
      );
    const game = JSON.parse(state),
      seat = engine.currentPlayer(state),
      player = game.players[seat];
    player.units.push({ id: 99, unit_type: { Leader: 'SunTzu' }, position: player.cities[0].position });
    player.resources.mood_tokens = 1;
    player.resources.culture_tokens = 1;
    state = JSON.stringify(game);
    c.setPlayer(seat);
    await c.load(engine.stripSecret(state, seat));
    const ability = app.session().view!.specialActions!.find((a) => a.name === 'The Art of War')!;
    c.submit(ability.action, ability.cost);
    state = engine.tryMove(state, app.sent[0], seat);
    await c.load(engine.stripSecret(state, seat));
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().automaticPayment, false);
    assert.equal(app.session().pending, false);
    assert.equal(app.session().view!.decision!.fields[0].choices!.length, 2);
    assert.equal(JSON.parse(state).players[seat].resources.mood_tokens, 1);
    assert.equal(JSON.parse(state).players[seat].resources.culture_tokens, 1);
  } finally {
    app.close();
  }
});

test('Expansion keeps its new settler and destination selected across repeated platform metadata and snapshots', async () => {
  let submitted = '';
  const controller = new Controller(
    {
      move: (move: string) => {
        submitted = move;
        return true;
      },
      replaceLog: () => {},
      fetchState: () => {},
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
  );
  let session: Session;
  const off = controller.session.subscribe((s) => {
    session = s;
  });
  try {
    const raw = JSON.parse(
      await engine.init(2, [], { civilization: 'Random' }, 'clash-preview-20260927', {}),
    );
    const seat = engine.currentPlayer(JSON.stringify(raw));
    raw.players[seat].civilization = 'China';
    raw.players[seat].advances.push('Husbandry');
    raw.players[seat].special_advances = ['Expansion'];
    let state = JSON.stringify(raw);
    const load = () => controller.load(engine.stripSecret(state, seat));
    controller.setPlayer(seat);
    await load();
    const city = session!.city!;
    const oldIds = session!.view!.units!.map((u) => u.id);
    controller.openCities(city, 'recruit');
    controller.setRecruits({ settlers: 1 });
    assert.ok(session!.recruitPreview?.action);
    controller.submit(session!.recruitPreview!.action);
    state = engine.tryMove(state, submitted, seat);
    await load();
    const newSettler = session!.view!.units!.find((u) => !oldIds.includes(u.id))!;
    assert.equal(session!.mode, 'settlers');
    assert.deepEqual(session!.selectedUnits, [newSettler.id]);
    assert.ok(session!.moveDestinations.length);
    const target = session!.moveDestinations[0].position;
    controller.selectTile(target);
    const selection = session!.moveDestination;
    controller.setPlayer(seat);
    await load();
    assert.equal(session!.mode, 'settlers');
    assert.deepEqual(session!.selectedUnits, [newSettler.id]);
    assert.equal(session!.moveTarget, target);
    assert.equal(session!.moveDestination, selection);
    // The city itself also resumes movement after intentionally closing the controls.
    controller.patch({ mode: 'overview', tilePanel: false });
    controller.selectTile(city, { kind: 'city', player: seat });
    assert.equal(session!.mode, 'settlers');
    assert.ok(session!.moveDestinations.length);
    // Duplicate snapshots must not acknowledge an action the server has not applied.
    controller.submit(session!.moveDestinations[0].action);
    await load();
    assert.equal(session!.pending, true);
  } finally {
    off();
    controller.destroy();
  }
});

test('unit decisions open a hex without picking a casualty and lock while a move is pending', async () => {
  const app = paymentController(),
    c = app.controller;
  const state = fixture('incidents/pandemics/black_death.outcome');
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(state, 0));
    c.selectTile('C2', { kind: 'units', player: 0 });
    assert.deepEqual(app.session().decisionSelection, []);
    assert.equal(app.session().decisionPosition, 'C2');
    c.selectTile('C2', { kind: 'unit', player: 0, unit: 3 });
    assert.deepEqual(app.session().decisionSelection, []);
    c.selectDecisionOption(2);
    assert.deepEqual(app.session().decisionSelection, [2]);
    assert.deepEqual(app.sent, []);
    c.selectTile('C2', { kind: 'unit', player: 1, unit: 0 });
    assert.deepEqual(app.session().decisionSelection, [2]);
    c.patch({ pending: true });
    c.selectTile('C2', { kind: 'decision', decisionIndex: 0 });
    assert.deepEqual(app.session().decisionSelection, [2]);
    c.patch({ pending: false });
    c.setPlayer(1);
    await c.load(engine.stripSecret(state, 1));
    assert.deepEqual(app.session().decisionSelection, []);
    c.selectTile('C2', { kind: 'decision', decisionIndex: 0 });
    assert.deepEqual(app.session().decisionSelection, []);
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('changing collection action keeps selected tiles and requotes the submitted action', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(fixture('advances/collect_free_economy'), 0));
    c.beginCollect();
    c.toggleChoice(app.session().view!.cities[0].choices[0]);
    const selected = structuredClone(app.session().selection);
    const normal = app.session().collectVariant;
    const free = app.session().view!.collectActions!.find((a) => a.name === 'Free Economy')!;
    const ordinaryAction = app.session().preview!.action;
    c.switchCollectVariant(free.value);
    assert.deepEqual(app.session().selection, selected);
    assert.notDeepEqual(app.session().preview!.action, ordinaryAction);
    assert.equal(app.sent.length, 0);
    c.switchCollectVariant(normal);
    assert.deepEqual(app.session().selection, selected);
    assert.deepEqual(app.session().preview!.action, ordinaryAction);
    c.switchCollectVariant(free.value);
    const freeAction = app.session().preview!.action;
    c.patch({ pending: true });
    c.switchCollectVariant(normal);
    assert.deepEqual(app.session().preview!.action, freeAction);
    c.patch({ pending: false });
    c.collect();
    assert.deepEqual(JSON.parse(app.sent[0]), freeAction);
  } finally {
    app.close();
  }
});

test('clicking another owned city on the map switches collection without spending an action', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(fixture('advances/collect_free_economy'), 0));
    c.beginCollect('A1');
    const free = app.session().view!.collectActions!.find((a) => a.name === 'Free Economy')!;
    c.switchCollectVariant(free.value);
    c.selectTile('B1');
    assert.ok(app.session().preview?.action);
    assert.ok(!app.session().view!.cities[0].choices.some((choice) => choice.position === 'C2'));
    c.patch({ collectionTile: 'B1' });
    c.selectTile('C2', { kind: 'city', player: 0 });
    assert.equal(app.session().mode, 'collect');
    assert.equal(app.session().city, 'C2');
    assert.equal(app.session().focus, 'C2');
    assert.equal(app.session().tilePanel, false);
    assert.deepEqual(app.session().collectVariant, free.value);
    assert.deepEqual(app.session().selection, []);
    assert.equal(app.session().preview, null);
    assert.equal(app.session().collectionTile, null);
    c.selectTile('B2');
    assert.ok(app.session().preview?.action, 'The newly selected city supplies the collection choices');
    c.selectTile('C1', { kind: 'city', player: 1 });
    assert.equal(app.session().city, 'C2', 'An opponent city cannot become the collecting city');
    c.patch({ pending: true });
    c.selectTile('A1');
    assert.equal(app.session().city, 'C2', 'Pending actions block city switching');
    c.patch({ pending: false });
    c.selectTile('A1');
    assert.equal(app.session().city, 'A1', 'Clicking the city hex also switches collection');
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('Sports chooses only an eligible map city and does not spend an action until confirmed', async () => {
  const { groupAbilities } = await import('./abilities.ts');
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(fixture('advances/increase_happiness_sports'), 0));
    const groups = groupAbilities(app.session().view!.specialActions);
    const sports = groups.filter((g) => g.offer.name === 'Sports');
    assert.equal(sports.length, 1);
    assert.equal(sports[0].offers.length, 3);
    c.chooseAbility(sports[0].key);
    c.selectTile('A1'); // Happy: ineligible.
    assert.equal(app.session().abilityCity, null);
    c.selectTile('B1', { kind: 'city', player: 0 });
    assert.equal(app.session().abilityCity, 'B1');
    c.selectTile('C2', { kind: 'unit', player: 0, unit: 0 });
    assert.equal(app.session().abilityCity, 'C2', 'Units on a city do not hijack the ability selection');
    assert.equal(app.session().tilePanel, false);
    assert.equal(app.session().mode, 'overview');
    assert.equal(app.sent.length, 0);
    c.patch({ pending: true });
    c.selectTile('B3');
    assert.equal(app.session().abilityCity, 'C2');
    c.patch({ pending: false, abilitiesOpen: false });
    assert.equal(app.session().abilityChoice, null);
    assert.equal(app.session().abilityCity, null);
    c.chooseAbility(sports[0].key);
    c.selectTile('B3');
    c.submit(sports[0].offers.find((o) => o.position === app.session().abilityCity)!.action);
    assert.deepEqual(JSON.parse(app.sent[0]), { Playing: { Custom: { action: 'Sports', city: 'B3' } } });
  } finally {
    app.close();
  }
});

test('opponent recap steps animate and pause, replay stays within its turn, and completion stays open', (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = paymentController(),
    c = app.controller;
  try {
    const frames = [
      { cursor: 1, actor: null, title: 'Game setup' },
      { cursor: 2, actor: 0, ended_turn: true },
      { cursor: 3, actor: 1, title: 'Move' },
      { cursor: 4, actor: 1, ended_turn: true },
      { cursor: 5, actor: 0, title: 'Collect' },
    ].map((f) => ({ players: [], tiles: [], age: 1, round: 1, ended_turn: false, title: '', ...f }));
    c.patch({
      game: {
        state: 'Playing',
        players: [],
        map: { tiles: [] },
        current_player_index: 0,
        actions_left: 3,
        age: 1,
        round: 1,
        log_index: 0,
        board_history: { id: 'recap-test', frames },
      },
      seat: 0,
    });
    c.replayLastTurn();
    assert.equal(app.session().playback!.index, 1);
    assert.equal(app.session().playback!.end, 3);
    assert.equal(app.session().playback!.playing, false);
    c.stepPlayback(1);
    assert.equal(app.session().playback!.index, 2);
    assert.equal(app.session().playback!.animate, true);
    assert.equal(app.session().playback!.playing, false);
    assert.deepEqual(app.preferences.at(-1), { name: 'replayAutoplay', value: false });
    t.mock.timers.tick(10000);
    assert.equal(app.session().playback!.index, 2, 'manual steps wait indefinitely');
    c.stepPlayback(-1);
    c.stepPlayback(-1);
    assert.equal(app.session().playback!.index, 1, 'Back stops at the start of this turn');
    c.togglePlayback();
    assert.equal(app.session().replayAutoplay, true);
    t.mock.timers.tick(2499);
    assert.equal(app.session().playback!.index, 1, 'autoplay leaves time to read');
    t.mock.timers.tick(1);
    assert.equal(app.session().playback!.index, 2);
    t.mock.timers.tick(2500);
    assert.equal(app.session().playback!.index, 3);
    assert.equal(app.session().playback!.playing, false, 'stays open at completion');
    t.mock.timers.tick(10000);
    assert.equal(app.session().playback!.index, 3);
    c.restartPlayback();
    assert.equal(app.session().playback!.index, 1, 'Replay does not restart the whole game');
    c.endPlayback();
    c.startPlayback(true, 1);
    c.setReplayAutoplay(false);
    assert.equal(app.session().playback!.playing, false);
    c.setPreferences({ replayAutoplay: false });
    t.mock.timers.tick(3000);
    assert.equal(app.session().playback!.index, 1, 'delayed preference acknowledgement keeps recap paused');
    c.endPlayback();
    c.setPreferences({ replayAutoplay: false });
    c.startPlayback(true, 1);
    assert.equal(app.session().playback!.playing, false, 'automatic catch-up obeys manual preference');
    c.stepPlayback(1);
    c.patch({ reducedMotion: true });
    c.stepPlayback(1);
    assert.equal(app.session().playback!.animate, false);
    assert.deepEqual(app.sent, [], 'replay never submits a game move');
  } finally {
    app.close();
  }
});

test('end-of-age razing stays optional by default; opting out keeps every city once', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const raw = fixture('status_phase/raze_city_decline');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().view!.decision!.name, 'Raze city');
    assert.equal(app.sent.length, 0);
    c.setSkipRazeCity(true);
    assert.deepEqual(app.preferences, [{ name: 'skipRazeCity', value: true }]);
    assert.equal(app.sent.length, 1);
    assert.deepEqual(JSON.parse(app.sent[0]), { Response: { SelectPositions: [] } });
    await c.load(engine.stripSecret(raw, 0));
    c.setPreferences({ skipRazeCity: true });
    assert.equal(app.sent.length, 1, 'duplicate snapshots and preferences never send twice');
    const next = engine.tryMove(raw, app.sent[0], 0);
    assert.deepEqual(JSON.parse(next).players[0].cities, JSON.parse(raw).players[0].cities);
    await c.load(engine.stripSecret(next, 0));
    assert.equal(app.sent.length, 1, 'does not skip an opponent decision');
    const afterOpponent = engine.tryMove(next, app.sent[0], 1);
    await c.load(engine.stripSecret(afterOpponent, 0));
    assert.notEqual(app.session().view!.decision?.name, 'Raze city');
    assert.equal(app.sent.length, 1, 'does not skip the next end-of-age decision');
  } finally {
    app.close();
  }
});

test('saved skip preference works when state arrives later, but never for spectators or analysis', async () => {
  for (const mode of ['play', 'spectator', 'analysis']) {
    const app = paymentController(),
      c = app.controller;
    try {
      c.setPreferences({ skipRazeCity: true, analysis: mode === 'analysis' });
      if (mode !== 'spectator') c.setPlayer(0);
      await c.load(
        engine.stripSecret(fixture('status_phase/raze_city_decline'), mode === 'spectator' ? undefined : 0),
      );
      assert.equal(app.sent.length, mode === 'play' ? 1 : 0, mode);
    } finally {
      app.close();
    }
  }
});

test('failed automatic razing skip stays available manually and is never retried in a loop', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    const raw = engine.stripSecret(fixture('status_phase/raze_city_decline'), 0);
    await c.load(raw);
    app.reject();
    c.setSkipRazeCity(true);
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().pending, false);
    assert.ok(app.session().error);
    c.patch({ error: '' });
    c.setPreferences({ skipRazeCity: true });
    await c.load(raw);
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().view!.decision!.name, 'Raze city');
  } finally {
    app.close();
  }
});

test('replay postpones automatic razing skip until returning to the game', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(fixture('status_phase/raze_city_decline'), 0));
    c.patch({
      playback: {
        frame: null,
        index: 0,
        total: 1,
        start: 0,
        end: 0,
        range: 'all',
        automatic: false,
        playing: false,
        animate: false,
      },
    });
    c.setSkipRazeCity(true);
    assert.equal(app.sent.length, 0);
    c.endPlayback();
    assert.equal(app.sent.length, 1);
  } finally {
    app.close();
  }
});

test('Redo restores an undone collection, remains private to the active seat, and clears on a different move', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    let raw = fixture('advances/collect_free_economy');
    c.setPlayer(0);
    const action = {
      Playing: {
        Collect: {
          city_position: 'C2',
          collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
          action_type: 'Collect',
        },
      },
    };
    raw = engine.tryMove(raw, JSON.stringify(action), 0);
    const collected = JSON.parse(raw);
    raw = engine.tryMove(raw, JSON.stringify('Undo'), 0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().view!.canRedo, true);
    assert.equal(JSON.parse(engine.webView(engine.stripSecret(raw, 1), 1)).canRedo, false);
    c.submit('Redo');
    assert.equal(app.sent.at(-1), '"Redo"');
    raw = engine.tryMove(raw, app.sent.at(-1)!, 0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().view!.canRedo, false);
    assert.deepEqual(JSON.parse(raw).players[0].resources, collected.players[0].resources);
    assert.equal(JSON.parse(raw).board_history.frames.at(-1).title, 'Collect');
    assert.equal(app.session().view!.canUndo, true);
    raw = engine.tryMove(raw, JSON.stringify('Undo'), 0);
    raw = engine.tryMove(
      raw,
      JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { food: 1, ideas: 1 } } } }),
      0,
    );
    assert.equal(JSON.parse(engine.webView(engine.stripSecret(raw, 0), 0)).canRedo, false);
  } finally {
    app.close();
  }
});

test('battle playback queues rounds, slows autoplay, keeps manual results and cancels on seek or skip', (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('combat/retreat_no.outcome1'));
    const base = {
      actor: 0,
      ended_turn: false,
      title: 'Battle',
      age: 1,
      round: 1,
      tiles: game.map.tiles,
      players: game.players,
    };
    game.board_history = {
      id: 'battle-controller',
      frames: [0, 1, 2].map((cursor) => ({ ...base, cursor })),
    };
    c.patch({ game, seat: 0 });
    c.startPlayback();
    c.stepPlayback(1);
    assert.equal(app.session().battles![0].combat.round, 1);
    assert.equal(app.session().battleAnimate, true);
    t.mock.timers.tick(10000);
    assert.equal(app.session().battles!.length, 1, 'manual result remains readable');
    assert.equal(app.session().playback!.index, 1);
    c.togglePlayback();
    t.mock.timers.tick(3000);
    assert.equal(app.session().playback!.index, 1, 'battle gets longer than the ordinary 2.5 seconds');
    t.mock.timers.tick(2000);
    assert.equal(app.session().playback!.index, 2);
    assert.equal(app.session().battles![0].combat.result, 'Greece wins');
    c.stepPlayback(-1);
    assert.equal(app.session().battleAnimate, false, 'Back inspects without reroll animation');
    assert.equal(app.session().battles![0].combat.result, undefined, 'no future result');
    c.patch({ reducedMotion: true });
    c.stepPlayback(1);
    assert.equal(app.session().battleAnimate, false);
    c.endPlayback();
    assert.deepEqual(app.session().battles, []);
    t.mock.timers.tick(20000);
    assert.deepEqual(app.session().battles, []);
  } finally {
    app.close();
  }
});

test('the first battle recorded in an older game animates live and stores only public battlefield metadata', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    let raw = fixture('combat/ship_combat');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().battles?.length ?? 0, 0);
    const expected = JSON.parse(fixture('combat/ship_combat.outcome'));
    const action = expected.log[0].rounds[0].turns[0].actions[0].action;
    raw = engine.tryMove(raw, JSON.stringify(action), 0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().battles!.length, 1);
    assert.equal(app.session().battles![0].combat.attacker.value, 12);
    const combat = JSON.parse(raw).board_history.frames.at(-1).combat;
    assert.deepEqual(combat, {
      round: 1,
      attacker: { player: 0, position: 'C3' },
      defender: { player: 1, position: 'D2' },
    });
    assert.deepEqual(JSON.parse(engine.stripSecret(raw, 1)).board_history.frames.at(-1).combat, combat);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().battles!.length, 1, 'refresh must not enqueue the battle twice');
  } finally {
    app.close();
  }
});
