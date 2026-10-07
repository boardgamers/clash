import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { build } from 'esbuild';
import type { Controller as ControllerType } from './controller';
import type { Session } from './types';
import { contextualCards, activeCollectionCard } from './contextual-cards.ts';
import { requiredRecruitDiscards } from './recruit-discards.ts';
import { happinessPreview, happinessCities } from './happiness.ts';
import { frameDetails } from './playback.ts';
import { frameResources } from './resource-playback.ts';

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
  const settings: { name: string; value: unknown }[] = [];
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
      updateSetting: (name: string, value: unknown) => {
        settings.push({ name, value });
        return accept;
      },
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
    settings,
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

test('finishing movement from another snapshot clears its controls before selecting a city', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('movement/movement'));
    game.players[0].civilization = 'Maya';
    game.players[0].advances.push('Tactics');
    game.players[0].units = [{ id: 9, position: 'C2', unit_type: { Leader: 'Pakal' } }];
    game.players[0].recruited_leaders = ['Pakal'];
    game.players[0].cities.find((city: any) => city.position === 'C2').activations = 0;
    game.actions_left = 2;
    let raw = JSON.stringify(game);
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    c.openUnits([9]);
    assert.equal(app.session().mode, 'settlers');
    assert.equal(app.session().unitPosition, 'C2');
    // No local pending flag: the move can finish in another tab or after reconnecting.
    raw = engine.tryMove(raw, JSON.stringify({ Movement: 'Stop' }), 0);
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.session().mode, 'overview');
    assert.equal(app.session().unitPosition, null);
    assert.deepEqual(app.session().selectedUnits, []);
    assert.deepEqual(app.session().moveDestinations, []);
    c.selectTile('C2', { kind: 'city', player: 0 });
    assert.equal(app.session().mode, 'overview');
    assert.equal(app.session().tilePanel, true);
    assert.equal(app.session().city, 'C2');
    c.openCities('C2', 'build');
    assert.equal(app.session().mode, 'city');
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('a new turn clears a previously opened movement panel', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('movement/movement'));
    game.state = 'Playing';
    game.players[0].advances.push('Tactics');
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.openUnits([0]);
    game.round += 1;
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    assert.equal(app.session().mode, 'overview');
    assert.deepEqual(app.session().selectedUnits, []);
    c.selectTile('C2');
    assert.equal(app.session().tilePanel, true);
  } finally {
    app.close();
  }
});

test('movement origins and next group select the whole compatible group without submitting', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('movement/movement'));
    g.players[0].advances.push('Tactics');
    g.players[0].units.push({ id: 8, position: 'B2', unit_type: 'Settler' });
    g.players[0].next_unit_id = 9;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    c.patch({ focus: 'C2', selectedUnits: [] });
    c.openSettlers();
    assert.deepEqual(app.session().selectedUnits, [0, 1, 2, 3, 5, 6, 7]);
    c.focusUnitPosition('B2');
    assert.deepEqual(app.session().selectedUnits, [4, 8]);
    c.selectUnits([4]);
    assert.deepEqual(app.session().selectedUnits, [4], 'individual deselection still works');
    c.selectTile('B2');
    assert.deepEqual(app.session().selectedUnits, [4, 8], 'selecting the same origin reselects everyone');
    c.selectUnits([]);
    c.selectTile('C2', { kind: 'unit', player: 0, unit: 0 });
    assert.deepEqual(app.session().selectedUnits, [0, 1, 2, 3, 5, 6, 7]);
    c.patch({ pending: true });
    c.focusUnitPosition('B2');
    assert.equal(app.session().unitPosition, 'C2');
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('movement group defaults exclude blocked units and keep ship passengers aboard', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('movement/movement'));
    g.players[0].advances.push('Tactics');
    g.state.Movement.moved_units = [0];
    g.players[0].units.push(
      { id: 8, position: 'C3', unit_type: 'Ship', carried_units: [{ id: 9, unit_type: 'Infantry' }] },
      { id: 10, position: 'C3', unit_type: 'Ship' },
    );
    g.players[0].next_unit_id = 11;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    c.openUnits([]);
    c.focusUnitPosition('C2');
    assert.deepEqual(app.session().selectedUnits, [1, 2, 3, 5, 6, 7]);
    c.focusUnitPosition('C3');
    assert.deepEqual(app.session().selectedUnits, [8, 10]);
    assert(!app.session().selectedUnits.includes(9));
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('clicking another movable group switches selection, while legal destinations keep their move', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('movement/movement'));
    g.players[0].advances.push('Tactics');
    g.map.tiles.push(['E2', 'Fertile'], ['E3', 'Fertile']);
    g.players[0].units.push({ id: 8, position: 'E2', unit_type: 'Infantry' });
    g.players[0].next_unit_id = 9;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    c.openUnits([4]);
    assert.ok(app.session().moveDestinations.some((d) => d.position === 'C2'));
    assert.ok(!app.session().moveDestinations.some((d) => d.position === 'E2'));
    c.selectTile('E2', { kind: 'unit', player: 0, unit: 8 });
    assert.deepEqual(app.session().selectedUnits, [8], 'Not a destination: select its own units instead');
    assert.equal(app.session().unitPosition, 'E2');
    assert.equal(app.session().mode, 'settlers');
    assert.ok(app.session().moveDestinations.length, 'The new group shows its own destinations');
    c.selectTile('B2');
    assert.deepEqual(app.session().selectedUnits, [4]);
    c.selectTile('C2', { kind: 'units', player: 0 });
    assert.deepEqual(app.session().selectedUnits, [4], 'A legal destination is still a destination');
    assert.equal(app.session().moveTarget, 'C2');
    c.selectTile('E2');
    assert.deepEqual(app.session().selectedUnits, [8], 'A chosen target does not block switching groups');
    assert.equal(app.session().moveTarget, null);
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('turning off move confirmation submits plain map moves but still confirms attacks', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('movement/movement'));
    g.players[0].advances.push('Tactics');
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    assert.equal(app.session().confirmMoves, true);
    c.openUnits([4]);
    c.selectTile('C2');
    assert.equal(app.session().moveTarget, 'C2');
    assert.deepEqual(app.sent, [], 'Confirmation is on by default');
    c.setPreferences({ confirmMoves: false });
    c.focusUnitPosition('B2');
    const route = app.session().moveDestinations.find((d) => d.position === 'C2')!;
    c.selectTile('C2');
    assert.deepEqual(app.sent.map((m) => JSON.parse(m)), [route.action]);
    assert.equal(app.session().pending, true);
    c.patch({ pending: false });
    c.focusUnitPosition('C2');
    const army = [0, 1, 2, 3];
    c.selectUnits(army);
    const attack = app.session().moveDestinations.find((d) => d.position === 'C1');
    assert.ok(attack?.attack, 'C1 holds enemy units');
    c.selectTile('C1');
    assert.equal(app.session().moveTarget, 'C1');
    assert.equal(app.sent.length, 1, 'Attacks keep their confirmation');
    // Picking units in the panel with a target already chosen never submits on its own.
    c.selectUnits([0, 1], 'C1');
    c.selectUnits([4], 'C2');
    assert.equal(app.sent.length, 1);
  } finally {
    app.close();
  }
});

test('map happiness selects multiple cities, preserves them across variants, and confirms together', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const raw = fixture('advances/increase_happiness_voting');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    c.openCities('C2', 'happiness');
    assert.equal(app.session().mode, 'happiness');
    assert.deepEqual(app.session().happinessSteps, { C2: 1 });
    assert(!happinessCities(app.session()).includes('A1'), 'happy cities are not targets');
    c.selectTile('A1');
    c.selectTile('C1'); // Another player's city.
    assert.deepEqual(app.session().happinessSteps, { C2: 1 });
    c.selectTile('B3', { kind: 'unit', player: 0, unit: 3 });
    c.setHappinessCity('C2', 2);
    assert.deepEqual(app.session().happinessSteps, { C2: 2, B3: 1 });
    const quote = () => happinessPreview(app.session(), (input) => c.query(input));
    assert.deepEqual(quote().payment, { mood_tokens: 7 });
    const voting = app.session().view!.happinessActions!.findIndex((v) => v.name === 'Voting');
    assert(voting >= 0);
    c.switchHappinessVariant(voting);
    assert.deepEqual(app.session().happinessSteps, { C2: 2, B3: 1 });
    assert.deepEqual(quote().payment, { mood_tokens: 8 });
    c.selectTile('B3');
    assert.deepEqual(app.session().happinessSteps, { C2: 2 });
    c.selectTile('B3');
    c.patch({ pending: true });
    c.selectTile('C2');
    assert.deepEqual(app.session().happinessSteps, { C2: 2, B3: 1 });
    c.patch({ pending: false });
    await c.load(engine.stripSecret(raw, 0));
    assert.deepEqual(
      app.session().happinessSteps,
      { C2: 2, B3: 1 },
      'duplicate snapshots preserve selections',
    );
    assert.equal(app.sent.length, 0, 'map choices do not submit actions');
    c.submit(quote().action!);
    const after = engine.tryMove(raw, app.sent[0], 0);
    const game = JSON.parse(after);
    assert.equal(game.players[0].cities.find((city: any) => city.position === 'C2').mood_state, 'Happy');
    assert.equal(game.players[0].cities.find((city: any) => city.position === 'B3').mood_state, 'Neutral');
    assert.equal(game.players[0].resources.mood_tokens, 1);
    assert.equal(game.actions_left, JSON.parse(raw).actions_left, 'Voting remains a free action');
    await c.load(engine.stripSecret(after, 0));
    assert.deepEqual(app.session().happinessSteps, {});
    assert.equal(app.session().mode, 'overview');
  } finally {
    app.close();
  }
});

test('map happiness keeps Lawgiver distinct and blocks unaffordable selections', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/increase_happiness_voting'));
    const p = game.players[0];
    p.civilization = 'Babylonia';
    p.units = [{ id: 1, position: 'C2', unit_type: { Leader: 'Hammurabi' } }];
    p.resources = { mood_tokens: 1, culture_tokens: 1 };
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.beginHappiness();
    c.selectTile('C2');
    assert.deepEqual(app.session().happinessSteps, { C2: 2 });
    assert.equal(app.session().happinessLawgiver, 'C2');
    c.selectTile('B3');
    assert.deepEqual(app.session().happinessSteps, { C2: 2, B3: 1 });
    c.selectTile('B1');
    assert.deepEqual(app.session().happinessSteps, { C2: 2, B3: 1 });
    const quoted = happinessPreview(app.session(), (input) => c.query(input));
    assert.deepEqual(quoted.payment, { culture_tokens: 1, mood_tokens: 1 });
    assert(quoted.action);
    c.selectTile('C2');
    assert.equal(app.session().happinessLawgiver, null);
    assert.deepEqual(app.session().happinessSteps, { B3: 1 });
    c.setPlayer(1);
    assert.deepEqual(app.session().happinessSteps, {});
    assert.equal(app.sent.length, 0);
  } finally {
    app.close();
  }
});

test('recruitment asks for only the missing pieces, clears stale discards and pays the full cost', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(await engine.init(2, [], {}, 'recruit-discards', {}));
    const seat = game.current_player_index,
      p = game.players[seat],
      city = p.cities[0].position;
    const initialView = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat));
    const limit = initialView.cityActions[0].recruits.find(
      (r: { type: string }) => r.type === 'Settler',
    ).limit;
    p.resources = { food: 7, ore: 7, mood_tokens: 7, culture_tokens: 7 };
    p.units = Array.from({ length: limit - 1 }, (_, id) => ({ id, position: city, unit_type: 'Settler' }));
    p.units.push({ id: 50, position: city, unit_type: 'Infantry' });
    p.next_unit_id = 51;
    const raw = JSON.stringify(game);
    c.setPlayer(seat);
    await c.load(engine.stripSecret(raw, seat));
    c.openCities(city, 'recruit');
    c.setRecruits({ settlers: 1 });
    assert.deepEqual(requiredRecruitDiscards(app.session().view, city, app.session().recruits), []);
    assert.ok(app.session().recruitPreview);
    c.setRecruits({ settlers: 2 });
    const groups = requiredRecruitDiscards(app.session().view, city, app.session().recruits);
    assert.deepEqual(
      groups.map((g) => [g.type, g.count]),
      [['Settler', 1]],
    );
    assert.ok(groups[0].units.every((u) => u.type === 'Settler'));
    assert.equal(app.session().recruitPreview, null);
    assert.equal(app.session().error, '', 'the inline choice replaces an Invalid replacement error');
    c.toggleRecruitDiscard(50);
    assert.deepEqual(app.session().replacements, [], 'unrelated types cannot be discarded');
    c.toggleRecruitDiscard(0);
    assert.ok(app.session().recruitPreview);
    c.toggleRecruitDiscard(1);
    assert.deepEqual(app.session().replacements, [0], 'cannot discard more than necessary');
    c.setRecruits({ settlers: 1 });
    assert.deepEqual(app.session().replacements, []);
    assert.ok(app.session().recruitPreview);
    c.setRecruits({ settlers: 2 });
    assert.equal(app.session().recruitPreview, null);
    c.toggleRecruitDiscard(1);
    c.setRecruits({});
    assert.deepEqual(app.session().replacements, []);
    c.setRecruits({ settlers: 2 });
    c.toggleRecruitDiscard(0);
    assert.deepEqual(app.sent, [], 'selecting a discard does not submit recruitment');
    c.submit(app.session().recruitPreview!.action);
    const after = JSON.parse(engine.tryMove(raw, app.sent[0], seat));
    assert.equal(after.players[seat].resources.food, 3, 'both new settlers cost food');
    assert.equal(
      after.players[seat].units.filter((u: { unit_type: string }) => u.unit_type === 'Settler').length,
      limit,
    );
    assert.ok(!after.players[seat].units.some((u: { id: number }) => u.id === 0));
  } finally {
    app.close();
  }
});

test('selecting a new leader accounts for the current one without asking for unrelated discards', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    let raw = await engine.init(2, [], {}, 'recruit-leader-discard', {});
    const game = JSON.parse(raw),
      seat = game.current_player_index,
      p = game.players[seat],
      city = p.cities[0].position;
    p.resources = { food: 7, ore: 7, mood_tokens: 7, culture_tokens: 7 };
    raw = JSON.stringify(game);
    c.setPlayer(seat);
    await c.load(engine.stripSecret(raw, seat));
    c.openCities(city, 'recruit');
    const first = app.session().view!.cityActions[0].leaders![0].id;
    c.setRecruits({ leader: first });
    assert.deepEqual(app.session().replacements, []);
    raw = engine.tryMove(raw, JSON.stringify(app.session().recruitPreview!.action), seat);
    await c.load(engine.stripSecret(raw, seat));
    c.openCities(city, 'recruit');
    const existing = app.session().view!.units!.find((u) => typeof u.type === 'object')!;
    const next = app.session().view!.cityActions[0].leaders!.find((leader) => !leader.reason)!.id;
    c.setRecruits({ leader: next });
    assert.deepEqual(app.session().replacements, [existing.id]);
    assert.ok(app.session().recruitPreview);
    assert.deepEqual(requiredRecruitDiscards(app.session().view, city, app.session().recruits), []);
    assert.deepEqual(app.sent, []);
    c.setRecruits({ infantry: 1 });
    assert.deepEqual(app.session().replacements, []);
    assert.ok(app.session().recruitPreview);
  } finally {
    app.close();
  }
});

test('inspecting journal research keeps the current decision, selection, seat and preferences', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    const raw = await engine.init(2, [], {}, 'journal-research-reference', {});
    await c.load(engine.stripSecret(raw, 0));
    c.patch({ mode: 'collect', activityOpen: true, selectedAdvance: 'Storage', availableOnly: true });
    const before = app.session();
    const view = c.researchReferenceView(1)!;
    assert.equal(view.advances.length, 48);
    assert.ok(view.advances.some((a) => a.id === 'PublicEducation'));
    assert.equal(app.session(), before);
    assert.deepEqual(app.sent, []);
    assert.deepEqual(app.preferences, []);
  } finally {
    app.close();
  }
});

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
    c.toggleChoice(
      app.session().view!.cities.find((city) => city.position === app.session().city)!.choices[0],
    );
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

async function victoriousDefender() {
  const game = JSON.parse(await engine.init(2, [], {}, 'after-battle-cards', {}));
  const seat = game.current_player_index;
  const player = game.players[seat];
  player.action_cards = [11, 5]; // Great Ideas and Hero General
  player.resources.ideas = 0;
  player.cities[0].mood_state = 'Neutral';
  game.actions_left = 0; // Both are free actions, even after the last main action.
  const turn = game.log.at(-1).rounds.at(-1).turns.at(-1);
  const stats = {
    round: 1,
    battleground: 'Land',
    attacker: { position: 'B2', player: 2, present: { infantry: 1 }, losses: { infantry: 1 } },
    defender: { position: player.cities[0].position, player: seat, present: { infantry: 2 } },
    result: 'DefenderWins',
  };
  turn.actions = [{ action: 'StartTurn', player: seat, combat_stats: stats }];
  game.log_index = 1;
  return { game, seat, stats, turn };
}

test('post-battle cards are suggested for a barbarian defensive victory and stay optional', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const { game, seat } = await victoriousDefender();
    let raw = JSON.stringify(game);
    c.setPlayer(seat);
    await c.load(engine.stripSecret(raw, seat));
    assert.deepEqual(
      contextualCards(app.session().view, 'after-battle').map((o) => o.card.name),
      ['Great Ideas', 'Hero General'],
    );
    assert.deepEqual(app.sent, [], 'a suggestion must not play or discard the card automatically');
    c.playContextualCard(11, 'after-battle');
    c.playContextualCard(11, 'after-battle');
    assert.equal(app.sent.length, 1);
    raw = engine.tryMove(raw, app.sent[0], seat);
    await c.load(engine.stripSecret(raw, seat));
    assert.equal(JSON.parse(raw).players[seat].resources.ideas, 2);
    assert.equal(JSON.parse(raw).actions_left, 0);
    assert.ok(!contextualCards(app.session().view, 'after-battle').some((o) => o.card.id === 11));
  } finally {
    app.close();
  }
});

test('post-battle suggestions follow engine eligibility, turn timing and undo', async () => {
  const original = await victoriousDefender();
  const offers = (game: typeof original.game, seat: number | undefined = original.seat) =>
    contextualCards(
      JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat)),
      'after-battle',
    );
  for (const [label, mutate] of [
    [
      'loss',
      (g: typeof original.game) => {
        g.log.at(-1).rounds.at(-1).turns.at(-1).actions[0].combat_stats.result = 'AttackerWins';
      },
    ],
    [
      'draw',
      (g: typeof original.game) => {
        g.log.at(-1).rounds.at(-1).turns.at(-1).actions[0].combat_stats.result = 'Draw';
      },
    ],
    [
      'naval battle',
      (g: typeof original.game) => {
        g.log.at(-1).rounds.at(-1).turns.at(-1).actions[0].combat_stats.battleground = 'Sea';
      },
    ],
    [
      'unfinished Move action',
      (g: typeof original.game) => {
        g.state = { Movement: { movement_actions_left: 1 } };
      },
    ],
    [
      'undone battle',
      (g: typeof original.game) => {
        g.log_index = 0;
      },
    ],
    [
      'previous turn',
      (g: typeof original.game) => {
        g.log
          .at(-1)
          .rounds.at(-1)
          .turns.push({ turn_type: { Player: original.seat } });
        g.log_index = 0;
      },
    ],
    [
      'opponent’s turn',
      (g: typeof original.game) => {
        g.current_player_index = 1 - original.seat;
      },
    ],
    [
      'another player’s victory',
      (g: typeof original.game) => {
        g.log.at(-1).rounds.at(-1).turns.at(-1).actions[0].combat_stats.defender.player = 1 - original.seat;
      },
    ],
  ] as const) {
    const game = structuredClone(original.game);
    mutate(game);
    assert.deepEqual(offers(game), [], label);
  }
  const full = structuredClone(original.game);
  full.players[original.seat].resources.ideas = full.players[original.seat].resource_limit.ideas;
  assert.ok(!offers(full).some((o) => o.card.name === 'Great Ideas'), 'no suggestion at the ideas limit');
  const spectator = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(original.game))));
  assert.deepEqual(contextualCards(spectator, 'after-battle'), []);
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
    c.toggleChoice(
      app.session().view!.cities.find((city) => city.position === app.session().city)!.choices[0],
    );
    assert.ok(app.session().preview);
    const before = JSON.parse(state);
    c.collect();
    // This save has no playback history yet; its id comes from the secret seed,
    // so the first move waits for the server.
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().pending, true);
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 1);
    state = engine.tryMove(state, app.sent[0], 0);
    await c.load(engine.stripSecret(state, 0));
    assert.equal(app.sent.length, 2);
    // The automatic fee is shown as paid immediately.
    assert.equal(app.session().pending, false);
    assert.equal(app.session().automaticPayment, false);
    assert.equal(
      app.session().game!.players[0].resources!.mood_tokens,
      before.players[0].resources.mood_tokens - 1,
    );
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
    // Inspecting pieces never resumes movement until Move is explicitly opened.
    controller.patch({ mode: 'overview', tilePanel: false });
    controller.selectTile(city, { kind: 'city', player: seat });
    assert.equal(session!.mode, 'overview');
    assert.equal(session!.tilePanel, true);
    controller.selectTile(city, { kind: 'unit', player: seat, unit: newSettler.id });
    assert.equal(session!.mode, 'overview');
    controller.openSettlers();
    controller.selectTile(city, { kind: 'unit', player: seat, unit: newSettler.id });
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

test('unit decisions open a hex, pick only the clicked unit model and lock while a move is pending', async () => {
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
    const clicked = app
      .session()
      .view!.decision!.options.findIndex((o) => o.mapTarget?.kind === 'unit' && o.mapTarget.unit === 3);
    assert.ok(clicked >= 0);
    assert.deepEqual(app.session().decisionSelection, [clicked], 'the clicked model is chosen, not confirmed');
    assert.deepEqual(app.sent, []);
    c.selectTile('C2', { kind: 'unit', player: 0, unit: 3 });
    assert.deepEqual(app.session().decisionSelection, [], 'clicking it again deselects');
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

test('Heavy Earthquake structures toggle from building and city clicks and wait for confirmation', async () => {
  const app = paymentController(),
    c = app.controller;
  const state = engine.tryMove(
    fixture('incidents/earthquake/earthquake'),
    JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
    0,
  );
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(state, 0));
    const options = app.session().view!.decision!.options;
    const at = (position: string, name: string) => options.findIndex((o) => o.name === `${name} · ${position}`);
    c.selectTile('C2', { kind: 'city', player: 0 });
    assert.equal(app.session().decisionPosition, 'C2');
    assert.deepEqual(app.session().decisionSelection, [], 'a city with several structures only opens its list');
    c.selectTile('C2', { kind: 'city', player: 0, structure: 'Building:Temple' });
    c.selectTile('B2', { kind: 'city', player: 0 });
    assert.equal(app.session().decisionPosition, 'B2');
    assert.deepEqual(app.session().decisionSelection, [at('C2', 'Temple'), at('B2', 'City center')]);
    c.selectTile('C2', { kind: 'city', player: 0, structure: 'Building:Temple' });
    assert.deepEqual(app.session().decisionSelection, [at('B2', 'City center')], 'a second click deselects');
    c.selectTile('A1', { kind: 'city', player: 1, structure: 'CityCenter' });
    assert.deepEqual(app.session().decisionSelection, [at('B2', 'City center')]);
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('cultural influence opens a map mode where clicking the target building chooses it', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('base/cultural_influence_instant'));
    g.current_player_index = 0;
    g.players[1].cities[0].city_pieces = { temple: 1, academy: 1 };
    g.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
    g.players[0].resources.culture_tokens = 7;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    c.patch({ abilitiesOpen: true });
    c.chooseInfluence();
    assert.equal(app.session().abilitiesOpen, true);
    assert.equal(app.session().influenceMode, true);
    c.selectTile('C1', { kind: 'city', player: 1 });
    assert.equal(app.session().influencePosition, 'C1');
    assert.equal(app.session().influenceTarget, null, 'Temple and Academy: the city opens its list');
    assert.equal(app.session().tilePanel, false, 'map clicks do not open the tile inspector');
    c.selectTile('C1', { kind: 'city', player: 1, structure: 'Building:Temple' });
    const temple = app.session().view!.influence!.find((o) => o.name === 'Temple')!;
    assert.equal(app.session().influenceTarget, `C1/Temple/${temple.variant}`);
    c.selectTile('A1', { kind: 'city', player: 0 });
    assert.equal(app.session().influenceOrigin, 'A1');
    assert.deepEqual(app.sent, [], 'choosing a target never starts the attempt');
    c.patch({ abilitiesOpen: false });
    assert.equal(app.session().influenceMode, false);
    assert.equal(app.session().influenceTarget, null);
  } finally {
    app.close();
  }
});

test('Ballcourts preserves collection choices and invalidates an oversized draft when disabled', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/collect_free_economy'));
    const p = game.players[0];
    p.civilization = 'Maya';
    p.advances = [...new Set([...p.advances, 'Arts', 'Sports'])];
    p.resources.mood_tokens = 8;
    p.cities[0].mood_state = 'Neutral';
    p.cities[0].activations = 0;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.beginCollect(p.cities[0].position);
    const city = app.session().view!.cities.find((a) => a.position === app.session().city)!;
    assert.ok(city.ballcourts);
    c.toggleChoice(city.choices[0]);
    const selected = structuredClone(app.session().selection);
    const ordinary = app.session().preview!.action;
    c.setBallcourts(true);
    assert.deepEqual(app.session().selection, selected);
    assert.notDeepEqual(app.session().preview!.action, ordinary);
    c.setBallcourts(false);
    assert.deepEqual(app.session().selection, selected);
    assert.deepEqual(app.session().preview!.action, ordinary);
    c.setBallcourts(true);
    c.toggleChoice(city.choices.find((a) => a.position !== selected[0].position)!);
    const extra = structuredClone(app.session().selection);
    assert.equal(
      extra.reduce((n, a) => n + a.times, 0),
      city.capacity + 1,
    );
    const boosted = app.session().preview!.action;
    c.setBallcourts(false);
    assert.deepEqual(app.session().selection, extra);
    assert.equal(app.session().preview, null);
    assert.ok(app.session().error);
    c.collect();
    assert.deepEqual(app.sent, []);
    c.setBallcourts(true);
    assert.deepEqual(app.session().selection, extra);
    assert.deepEqual(app.session().preview!.action, boosted);
    assert.equal(app.session().error, '');
    c.collect();
    assert.deepEqual(JSON.parse(app.sent[0]), boosted);
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
    c.toggleChoice(
      app.session().view!.cities.find((city) => city.position === app.session().city)!.choices[0],
    );
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
    assert.equal(app.session().playback!.end, 2);
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
    assert.equal(app.session().playback!.index, 2);
    assert.equal(app.session().playback!.playing, false, 'stays open at completion');
    t.mock.timers.tick(10000);
    assert.equal(app.session().playback!.index, 2);
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

test('finished games open on the final state and stop an automatic recap already in progress', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const game = JSON.parse(await engine.init(2, [], {}, 'finished-recap', {}));
  const snapshot = {
    ended_turn: false,
    age: game.age,
    round: game.round,
    tiles: game.map.tiles,
    players: game.players.map(({ id, civilization, cities = [], units = [] }: any) => ({
      id,
      civilization,
      cities,
      units,
    })),
  };
  game.board_history = {
    id: 'finished-recap',
    frames: [
      { ...snapshot, cursor: 100, actor: 0, title: 'End turn', ended_turn: true },
      {
        ...snapshot,
        cursor: 101,
        actor: 1,
        title: 'Research',
        effects: [{ player: 1, kind: 'action', label: 'Drew an action card' }],
      },
    ],
  };
  const playing = JSON.stringify(game);
  const finished = JSON.stringify({ ...game, state: 'Finished' });
  for (const autoplay of [true, false]) {
    const app = paymentController(),
      c = app.controller;
    try {
      c.setPlayer(0);
      c.setPreferences({ replayAutoplay: autoplay });
      await c.load(finished);
      assert.equal(app.session().game!.state, 'Finished');
      assert.equal(app.session().playback ?? null, null, 'no automatic recap, even if autoplay is disabled');
      c.replayLastTurn();
      assert.equal(app.session().playback!.range, 'last-turn', 'manual replay remains available');
      c.seekPlayback(100);
      assert.equal(app.session().playback!.range, 'all', 'full history remains available');
      await c.load(JSON.stringify({ ...game, state: 'Finished', actions_left: 0 }));
      assert.equal(app.session().playback!.range, 'all', 'refreshing a finished game keeps manual replay');
      assert.deepEqual(app.sent, []);
    } finally {
      app.close();
    }
    const app2 = paymentController(),
      c2 = app2.controller;
    try {
      c2.setPlayer(0);
      c2.setPreferences({ replayAutoplay: autoplay });
      await c2.load(playing);
      assert.equal(app2.session().playback!.automatic, true, 'ongoing games still catch up');
      await c2.load(finished);
      assert.equal(app2.session().playback, null, 'completion closes playing or paused automatic recap');
      t.mock.timers.tick(10000);
      assert.equal(app2.session().playback, null, 'the recap timer cannot restart after completion');
      assert.deepEqual(app2.sent, []);
    } finally {
      app2.close();
    }
  }
});

test('explicit keep-cities choice saves engine settings before answering the current decision', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const raw = fixture('status_phase/raze_city_decline');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    c.setSettings({});
    c.setSkipRazeCity(true);
    assert.deepEqual(app.settings, [{ name: 'skipRazeCity', value: true }]);
    assert.deepEqual(app.preferences, []);
    assert.equal(app.sent.length, 0, 'wait for server acknowledgement');
    c.setSettings({ skipRazeCity: true });
    assert.deepEqual(JSON.parse(app.sent[0]), { Response: { SelectPositions: [] } });
    c.setSettings({ skipRazeCity: true });
    await c.load(engine.stripSecret(raw, 0));
    assert.equal(app.sent.length, 1, 'never auto-submit from repeated settings or state');
  } finally {
    app.close();
  }
});

test('saved engine settings never make the viewer automatically submit game moves', async () => {
  for (const mode of ['play', 'spectator', 'analysis']) {
    const app = paymentController(),
      c = app.controller;
    try {
      c.setPreferences({ analysis: mode === 'analysis' });
      if (mode !== 'spectator') c.setPlayer(0);
      c.setSettings({ skipRazeCity: true });
      await c.load(
        engine.stripSecret(fixture('status_phase/raze_city_decline'), mode === 'spectator' ? undefined : 0),
      );
      c.setPreferences({ sound: false });
      assert.equal(app.session().skipRazeCity, true, 'unrelated preferences leave saved setting alone');
      assert.deepEqual(app.sent, []);
      assert.deepEqual(app.settings, []);
    } finally {
      app.close();
    }
  }
});

test('legacy opt-in migrates once regardless of delivery order, preserving explicit opt-outs', async () => {
  for (const order of [
    'settings-first',
    'preferences-first',
    'state-first',
    'explicit-false',
    'analysis',
    'spectator',
  ]) {
    const app = paymentController(),
      c = app.controller;
    try {
      const prefs = () => c.setPreferences({ skipRazeCity: true, analysis: order === 'analysis' });
      const settings = () => c.setSettings(order === 'explicit-false' ? { skipRazeCity: false } : {});
      const load = async () => {
        if (order !== 'spectator') c.setPlayer(0);
        await c.load(
          engine.stripSecret(
            fixture('status_phase/raze_city_decline'),
            order === 'spectator' ? undefined : 0,
          ),
        );
      };
      if (order === 'state-first') {
        await load();
        settings();
        prefs();
      } else if (order === 'settings-first') {
        settings();
        prefs();
        await load();
      } else {
        prefs();
        settings();
        await load();
      }
      prefs();
      settings();
      assert.equal(
        app.settings.length,
        ['explicit-false', 'analysis', 'spectator'].includes(order) ? 0 : 1,
        order,
      );
      assert.deepEqual(app.sent, []);
    } finally {
      app.close();
    }
  }
});

test('failed setting update leaves the current razing decision available', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(0);
    await c.load(engine.stripSecret(fixture('status_phase/raze_city_decline'), 0));
    app.reject();
    c.setSkipRazeCity(true);
    c.setSettings({ skipRazeCity: true });
    assert.deepEqual(app.sent, []);
    assert.equal(app.session().view!.decision!.name, 'Raze city');
    assert.ok(app.session().error);
  } finally {
    app.close();
  }
});

test('engine skips an offline player’s end-of-age raze choice after another player moves', () => {
  const raw = fixture('status_phase/raze_city_decline');
  assert.deepEqual(engine.playerSettings(raw, 1), {}, 'old saves have no explicit choice');
  const saved = engine.setPlayerSettings(raw, 1, { skipRazeCity: true });
  assert.deepEqual(engine.playerSettings(saved, 1), { skipRazeCity: true });
  assert.equal(
    engine.currentPlayer(saved),
    engine.currentPlayer(raw),
    'settings alone cannot advance the turn',
  );
  const answer = JSON.stringify({ Response: { SelectPositions: [] } });
  const after = engine.tryMove(saved, answer, 0);
  assert.notEqual(JSON.parse(engine.webView(after, engine.currentPlayer(after))).decision?.name, 'Raze city');
  assert.deepEqual(JSON.parse(after).players[1].cities, JSON.parse(raw).players[1].cities);
  assert.deepEqual(JSON.parse(after).players[1].resources, JSON.parse(raw).players[1].resources);
  const frames = JSON.parse(after).board_history.frames;
  assert.equal(frames.at(-1).actor, 1, 'the offline player’s response is recorded for replay');
  assert.equal(frames.at(-1).title, 'Raze city');
  const optedOut = engine.setPlayerSettings(saved, 1, { skipRazeCity: false });
  const manual = engine.tryMove(optedOut, answer, 0);
  assert.equal(engine.currentPlayer(manual), 1);
  assert.equal(JSON.parse(engine.webView(manual, 1)).decision.name, 'Raze city');
  assert.throws(() => engine.setPlayerSettings(raw, 99, { skipRazeCity: true }));
  assert.throws(() => engine.setPlayerSettings(raw, 1, { skipRazeCity: 'true' }));
  const publicState = JSON.parse(engine.stripSecret(saved, 0));
  assert.equal(publicState.players[1].settings, undefined, 'private setting stays private');
  assert.deepEqual(
    engine.playerSettings(engine.createAnalysis(saved, { to: engine.logLength(saved) }), 1),
    {},
  );
});

test('player settings survive undoing and redoing an unrelated game move', () => {
  const raw = fixture('advances/collect_free_economy');
  const action = {
    Playing: {
      Collect: {
        city_position: 'C2',
        collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
        action_type: 'Collect',
      },
    },
  };
  const moved = engine.tryMove(raw, JSON.stringify(action), 0);
  const saved = engine.setPlayerSettings(moved, 0, { skipRazeCity: true });
  const undone = engine.tryMove(saved, JSON.stringify('Undo'), 0);
  assert.deepEqual(engine.playerSettings(undone, 0), { skipRazeCity: true });
  const redone = engine.tryMove(undone, JSON.stringify('Redo'), 0);
  assert.deepEqual(engine.playerSettings(redone, 0), { skipRazeCity: true });
});

test('Huns can start city-only movement and select other Nomad cities directly on the map', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    let g = JSON.parse(
      await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'nomads-exploration', {}),
    );
    for (const civilization of ['Huns', 'Rome']) {
      g = JSON.parse(
        engine.tryMove(
          JSON.stringify(g),
          JSON.stringify({ ChooseCivilization: civilization }),
          engine.currentPlayer(JSON.stringify(g)),
        ),
      );
    }
    g.players[0].civilization = 'Huns';
    g.players[0].advances.push('Storage');
    g.players[0].units = [];
    g.players[0].cities = ['C2', 'D1'].map((position) => ({ position, mood_state: 'Neutral' }));
    g.actions_left = 2;
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(g), 0));
    assert.equal(app.session().view!.units?.length, 0);
    assert.deepEqual(app.session().view!.nomadCities, ['C2', 'D1']);
    c.openSettlers();
    assert.equal(app.session().movingCity, 'C2');
    assert.ok(app.session().moveDestinations.length);
    c.selectTile('D1', { kind: 'city', player: 0 });
    assert.equal(app.session().movingCity, 'D1');
    assert.ok(app.session().moveDestinations.length);
    const dest = app.session().moveDestinations.find((d) => d.terrain !== 'Unexplored')!;
    assert.ok(dest);
    c.selectTile(dest.position);
    // A legal destination remains a move, rather than selecting the city beneath it.
    assert.ok(app.sent.length || app.session().moveDestination || app.session().moveTarget);
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

async function undoPreviewFixture() {
  const app = paymentController();
  const game = JSON.parse(fixture('advances/collect_free_economy'));
  game.board_history = { id: 'undo-preview-test', frames: [] };
  const action = {
    Playing: {
      Collect: {
        city_position: 'C2',
        collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
        action_type: 'Collect',
      },
    },
  };
  // Seed the fixture's public board history, as a live game already has.
  const before = engine.tryMove(
    engine.tryMove(JSON.stringify(game), JSON.stringify(action), 0),
    JSON.stringify('Undo'),
    0,
  );
  app.controller.setPlayer(0);
  await app.controller.load(engine.stripSecret(before, 0));
  app.controller.submit(action);
  const after = engine.tryMove(before, JSON.stringify(action), 0);
  await app.controller.load(engine.stripSecret(after, 0));
  return { ...app, before, after };
}

test('Undo immediately previews a confirmed earlier state, ignores stale snapshots, then reconciles', async () => {
  const app = await undoPreviewFixture();
  const c = app.controller;
  try {
    const before = JSON.parse(app.before),
      after = JSON.parse(app.after);
    assert.notDeepEqual(before.players[0].resources, after.players[0].resources);
    c.submit('Undo');
    assert.deepEqual(app.session().game!.players[0].resources, before.players[0].resources);
    assert.equal(app.session().game!.actions_left, before.actions_left);
    assert.equal(app.session().pending, false, 'the predicted undo does not wait for the server');
    assert.equal(app.session().view!.canUndo, false);
    assert.equal(app.session().view!.canRedo, true, 'Redo is offered immediately');
    c.submit('Undo');
    assert.equal(app.sent.length, 2, 'one collection and one undo, without duplicate submissions');
    const shown = app.session();
    await c.load(engine.stripSecret(app.after, 0));
    assert.equal(app.session(), shown, 'the old snapshot does not acknowledge or revert Undo');
    const undone = engine.tryMove(app.after, JSON.stringify('Undo'), 0);
    await c.load(engine.stripSecret(undone, 0));
    assert.equal(app.session(), shown, 'a matching confirmation changes nothing on screen');
    assert.equal(app.session().pending, false);
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(undone).players[0].resources);
    assert.equal(app.session().view!.canRedo, true);
  } finally {
    app.close();
  }
});

test('a rejected optimistic Undo restores the confirmed game without reverting preferences', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = await undoPreviewFixture();
  try {
    app.controller.submit('Undo');
    app.controller.patch({ dark: true });
    app.controller.handleError('Undo rejected');
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(app.after).players[0].resources);
    assert.equal(app.session().pending, false);
    assert.equal(app.session().dark, true);
    t.mock.timers.tick(8000);
    assert.equal(app.session().error, 'Undo rejected', 'the retry timer is cancelled on failure');
  } finally {
    app.close();
  }
});

test('an Undo timeout restores the confirmed state and a late confirmation still applies', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const app = await undoPreviewFixture();
  try {
    app.controller.submit('Undo');
    t.mock.timers.tick(8000);
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(app.after).players[0].resources);
    assert.equal(app.session().pending, true);
    const undone = engine.tryMove(app.after, JSON.stringify('Undo'), 0);
    await app.controller.load(engine.stripSecret(undone, 0));
    assert.equal(app.session().pending, false);
    assert.equal(app.session().error, '');
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(app.before).players[0].resources);
  } finally {
    app.close();
  }
});

test('Undo waits for the server after a seat change or when its target snapshot was never received', async () => {
  const app = await undoPreviewFixture();
  const fresh = paymentController();
  try {
    app.controller.setPlayer(1);
    app.controller.setPlayer(0);
    fresh.controller.setPlayer(0);
    await fresh.controller.load(engine.stripSecret(app.after, 0));
    for (const current of [app, fresh]) {
      current.controller.submit('Undo');
      assert.equal(current.session().pending, true);
      assert.deepEqual(
        current.session().game!.players[0].resources,
        JSON.parse(app.after).players[0].resources,
      );
      current.controller.handleError('Undo rejected');
    }
  } finally {
    app.close();
    fresh.close();
  }
});

test('Undo uses the new branch after undoing and choosing a different collection', async () => {
  const app = await undoPreviewFixture();
  try {
    app.controller.submit('Undo');
    let raw = engine.tryMove(app.after, JSON.stringify('Undo'), 0);
    await app.controller.load(engine.stripSecret(raw, 0));
    const wood = {
      Playing: {
        Collect: {
          city_position: 'C2',
          collections: [{ position: 'C2', pile: { wood: 1 }, times: 1 }],
          action_type: 'Collect',
        },
      },
    };
    app.controller.submit(wood);
    raw = engine.tryMove(raw, JSON.stringify(wood), 0);
    await app.controller.load(engine.stripSecret(raw, 0));
    app.controller.submit('Undo');
    assert.deepEqual(app.session().game!.players[0].resources, JSON.parse(app.before).players[0].resources);
    await app.controller.load(engine.stripSecret(engine.tryMove(raw, JSON.stringify('Undo'), 0), 0));
    assert.equal(app.session().pending, false);
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

test('real Taxes commands load as one replay action with public resources and per-city markers', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/taxes'));
    game.players[0].resources = { mood_tokens: 3 };
    let raw = JSON.stringify(game);
    for (const action of [
      { Playing: { Custom: { action: 'Taxes' } } },
      { Response: { Payment: [{ mood_tokens: 1 }] } },
      { Response: { ResourceReward: { food: 1, wood: 1, ore: 1, gold: 1 } } },
    ])
      raw = engine.tryMove(raw, JSON.stringify(action), 0);
    assert.equal(JSON.parse(raw).board_history.frames.length, 4);
    c.setPlayer(1);
    await c.load(engine.stripSecret(raw, 1));
    const visible = app.session().game!,
      frames = visible.board_history!.frames;
    assert.equal(frames.length, 2);
    assert.match(
      frameDetails(frames[0], frames[1], visible).caption,
      /Taxes · gained 1 food, 1 wood, 1 ore, 1 gold/,
    );
    assert.equal(frameResources(visible, frames[0].cursor, frames[1]).markers.length, 4);
    c.startPlayback();
    c.stepPlayback(1);
    assert.equal(app.session().playback!.index, 1);
    assert.equal(app.session().playback!.end, 1);
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('engine resolves consecutive offline raze choices and records responses that replay without settings', async () => {
  const game = JSON.parse(await engine.init(3, [], { civilization: 'Random' }, 'auto-multi', {}));
  game.round = 3;
  game.current_player_index = (game.starting_player_index + 2) % 3;
  game.actions_left = 0;
  let raw = JSON.stringify(game);
  for (let seat = 0; seat < 3; seat++) raw = engine.setPlayerSettings(raw, seat, { skipRazeCity: true });
  raw = engine.tryMove(raw, JSON.stringify({ Playing: 'EndTurn' }), engine.currentPlayer(raw));
  for (let i = 0; i < 2; i++) {
    const seat = engine.currentPlayer(raw);
    const decision = JSON.parse(engine.webView(raw, seat)).decision;
    assert.equal(decision.name, 'Free Advance', 'unrelated choices remain manual');
    raw = engine.tryMove(
      raw,
      JSON.stringify({ Response: { SelectAdvance: decision.options[0].value } }),
      seat,
    );
  }
  const seat = engine.currentPlayer(raw);
  const decision = JSON.parse(engine.webView(raw, seat)).decision;
  const advance = JSON.stringify({ Response: { SelectAdvance: decision.options[0].value } });
  const after = engine.tryMove(raw, advance, seat);
  assert.equal(
    JSON.parse(engine.webView(after, engine.currentPlayer(after))).decision.name,
    'Determine First Player',
    'all three raze choices are skipped, while the next choice remains manual',
  );
  const frames = JSON.parse(after).board_history.frames.filter((f: any) => f.title === 'Raze city');
  assert.equal(frames.length, 3);
  let manual = raw;
  for (let seat = 0; seat < 3; seat++)
    manual = engine.setPlayerSettings(manual, seat, { skipRazeCity: false });
  manual = engine.tryMove(manual, advance, seat);
  for (const frame of frames) {
    assert.equal(engine.currentPlayer(manual), frame.actor);
    manual = engine.tryMove(manual, JSON.stringify({ Response: { SelectPositions: [] } }), frame.actor);
  }
  for (const key of ['age', 'round', 'state', 'events', 'current_player_index', 'actions_left'])
    assert.deepEqual(JSON.parse(manual)[key], JSON.parse(after)[key], key);
});

test('toolbar city actions choose a fresh suitable city, while explicit map/city choices stay selected', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const game = JSON.parse(fixture('advances/collect_free_economy'));
    const p = game.players[0];
    p.advances = [
      'Farming',
      'Mining',
      'Storage',
      'Writing',
      'PublicEducation',
      'Bartering',
      'Myths',
      'Tactics',
    ];
    p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
    p.units = [];
    p.cities = [
      { position: 'A1', mood_state: 'Happy', city_pieces: { market: 0 } },
      { position: 'C2', mood_state: 'Happy', city_pieces: { academy: 0 } },
      {
        position: 'F2',
        mood_state: 'Happy',
        activations: 1,
        city_pieces: { temple: 0, market: 0, fortress: 0 },
      },
      { position: 'F5', mood_state: 'Angry', activations: 1, angry_activation: true },
    ];
    game.map.tiles = [];
    for (const col of 'ABCDEFG')
      for (let row = 1; row <= 6; row++) game.map.tiles.push([`${col}${row}`, 'Fertile']);
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.selectCity('F5');
    c.beginCollect();
    assert.equal(
      app.session().city,
      'C2',
      'unactivated Academy beats the equal-size city and used larger city',
    );
    assert.equal(app.session().focus, 'C2');
    assert.deepEqual(app.session().selection, [], 'do not preselect or spend resources');
    c.beginCollect('A1');
    assert.equal(app.session().city, 'A1', 'explicit city wins');
    c.selectCity('F2');
    c.openCities();
    assert.ok(['A1', 'C2'].includes(app.session().city!), 'building defaults to a usable unactivated city');
    c.openCities('F2', 'recruit');
    assert.equal(app.session().city, 'F2');
    c.openCities(undefined, 'recruit');
    assert.ok(['A1', 'C2'].includes(app.session().city!));
    assert.equal(app.session().focus, app.session().city);
    assert.deepEqual(app.sent, [], 'defaults never submit a move');
  } finally {
    app.close();
  }
});

test('influence confirmation carries through only the accepted upfront payments and stops for optional boosts', async () => {
  for (const drama of [false, true]) {
    const app = paymentController(),
      c = app.controller;
    try {
      const g = JSON.parse(fixture('base/cultural_influence_instant'));
      g.dice_roll_outcomes = [0];
      g.current_player_index = 0;
      g.players[1].cities[0].city_pieces = { temple: 1 };
      g.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
      g.players[0].civilization = 'Rome';
      g.players[0].resources.culture_tokens = 7;
      if (drama) g.players[0].advances = [...new Set([...g.players[0].advances, 'Arts', 'Theaters'])];
      c.setPlayer(0);
      let raw = JSON.stringify(g);
      await c.load(engine.stripSecret(raw, 0));
      const target = app
        .session()
        .view!.influence!.find((t) => t.position === 'C1' && t.name === 'Temple' && !!t.free === drama)!;
      const origin = target.origins!.find((o) => o.position === target.origin)!;
      c.startInfluence(origin.action, origin.actionPayment!, origin.payment);
      for (let i = 0; i < app.sent.length; i++) {
        assert.ok(i < 3, 'no optional purchases can follow the upfront quote');
        raw = engine.tryMove(raw, app.sent[i], 0);
        await c.load(engine.stripSecret(raw, 0));
      }
      assert.equal(app.sent.length, drama ? 3 : 2);
      assert.equal(app.session().view!.influenceContext!.stage, 'boost');
      assert.equal(app.session().pending, false);
      const count = app.sent.length;
      const payment = app
        .session()
        .view!.decision!.fields[0].choices!.find((p) => Object.values(p).some(Boolean))!;
      c.submit(c.query<{ action: any }>({ kind: 'decision', values: [], payments: [payment] }).action);
      raw = engine.tryMove(raw, app.sent[count], 0);
      await c.load(engine.stripSecret(raw, 0));
      assert.equal(app.session().toast, 'Cultural influence succeeded');
    } finally {
      app.close();
    }
  }
});

test('Shogunate has a visible card-only draft path independent of its free card-play allowance', async () => {
  let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'shogunate-feedback', {});
  raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: 'Japan' }), engine.currentPlayer(raw));
  raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: 'Rome' }), engine.currentPlayer(raw));
  const game = JSON.parse(raw),
    index = engine.currentPlayer(raw),
    p = game.players[index];
  p.advances = [...new Set([...p.advances, 'Tactics', 'Draft', 'Nationalism'])];
  p.resources.mood_tokens = 7;
  p.event_info = { ...p.event_info, 'Shogunate card': 'used' };
  p.cities[0].mood_state = 'Neutral';
  raw = JSON.stringify(game);
  const app = paymentController(),
    c = app.controller;
  try {
    c.setPlayer(index);
    await c.load(engine.stripSecret(raw, index));
    const offer = c.shogunateDraftOffers()[0];
    assert.equal(offer.position, p.cities[0].position);
    assert.deepEqual(offer.payment, { mood_tokens: 1 });
    c.beginShogunateDraft(offer.position);
    assert.equal(app.sent.length, 0, 'entry point prepares the confirmation');
    assert.equal(app.session().mode, 'city');
    assert.equal(app.session().cityTab, 'recruit');
    assert.equal(app.session().draftCard, true);
    assert.deepEqual(app.session().recruits, {});
    const quote = app.session().recruitPreview!;
    assert.ok(quote.action);
    const after = JSON.parse(engine.tryMove(raw, JSON.stringify(quote.action), index));
    assert.equal(after.players[index].event_info['Shogunate Draft'], 'used');
    assert.equal(after.players[index].event_info['Shogunate card'], 'used');
    assert.equal(after.actions_left, game.actions_left - 1);
    await c.load(engine.stripSecret(JSON.stringify(after), index));
    assert.deepEqual(c.shogunateDraftOffers(), []);
    p.advances = p.advances.filter((a: string) => a !== 'Nationalism');
    p.advances.push('Philosophy', 'Voting', 'CivilLiberties');
    await c.load(engine.stripSecret(JSON.stringify(game), index));
    const expensive = c.shogunateDraftOffers()[0];
    assert.deepEqual(expensive.payment, { mood_tokens: 2 });
    assert.equal(app.session().view?.cities[0].shogunateDraftCost, 2);
  } finally {
    app.close();
  }
});

test('pirate map guide switches incident players without moves and clears when collecting', async () => {
  const app = paymentController();
  const c = app.controller;
  try {
    const game = JSON.parse(fixture('incidents/pirates_spawn.outcome1'));
    game.events = [];
    c.setPlayer(0);
    await c.load(engine.stripSecret(JSON.stringify(game), 0));
    c.showSeaRoutes();
    c.showPirateSpawns();
    assert.equal(app.session().seaRoutes, false);
    assert.equal(app.session().pirateSpawns, true);
    c.setPirateSpawnPlayer(1);
    assert.equal(app.session().pirateSpawnPlayer, 1);
    c.setPirateSpawnPlayer(999);
    assert.equal(app.session().pirateSpawnPlayer, 1);
    assert.deepEqual(app.sent, []);
    c.beginCollect('A1');
    assert.equal(app.session().pirateSpawns, false);
  } finally {
    app.close();
  }
});

test('research confirmation pays Free Education once, or skips it, using the quoted choice', async () => {
  for (const buy of [true, false]) {
    const app = paymentController(),
      c = app.controller;
    try {
      const g = JSON.parse(fixture('advances/free_education'));
      g.players[0].resources.ideas = 6;
      let raw = JSON.stringify(g);
      c.setPlayer(0);
      await c.load(engine.stripSecret(raw, 0));
      const advance = app.session().view!.advances.find((a) => a.id === 'Irrigation')!;
      assert.ok(advance.action);
      const before = raw;
      assert.deepEqual(c.researchPlan(advance.action!), { eligible: true, affordable: true, combined: true });
      assert.equal(raw, before, 'quoting never changes the save');
      c.submitResearch(advance.action!, buy);
      for (let i = 0; i < app.sent.length; i++) {
        assert.ok(i < 2, 'only the research and its Free Education choice are submitted');
        raw = engine.tryMove(raw, app.sent[i], 0);
        await c.load(engine.stripSecret(raw, 0));
      }
      assert.equal(app.sent.length, 2);
      const p = JSON.parse(raw).players[0];
      assert.equal(
        p.resources.ideas,
        6 - (advance.action as any).Playing.Advance.payment.ideas - Number(buy),
      );
      assert.equal(p.resources.mood_tokens, 12 + Number(buy));
      assert.equal(app.session().view!.decision, null);
    } finally {
      app.close();
    }
  }
});

test('failed research confirmation cannot carry a Free Education payment into another move', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const g = JSON.parse(fixture('advances/free_education'));
    g.players[0].resources.ideas = 6;
    const raw = JSON.stringify(g);
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    const action = app.session().view!.advances.find((a) => a.id === 'Irrigation')!.action!;
    c.submitResearch(action, true);
    c.handleError('Research rejected');
    const after = engine.tryMove(raw, JSON.stringify(action), 0);
    await c.load(engine.stripSecret(after, 0));
    assert.equal(app.sent.length, 1);
    assert.equal(app.session().view!.decision!.origin!.Advance, 'FreeEducation');
  } finally {
    app.close();
  }
});

test('home-at-bottom changes from platform preferences never submit moves or write preferences back', () => {
  const app = paymentController(),
    c = app.controller;
  try {
    assert.equal(app.session().homeAtBottom, false);
    c.setPreferences({ homeAtBottom: true });
    assert.equal(app.session().homeAtBottom, true);
    c.setPreferences({ homeAtBottom: false });
    assert.equal(app.session().homeAtBottom, false);
    c.setPreferences({});
    assert.equal(app.session().homeAtBottom, false);
    assert.deepEqual(app.preferences, []);
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('recap navigation skips empty phases across multiple opponents while keeping real actions', () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const frames = [
      { cursor: 0, actor: 0, title: 'End turn', ended_turn: true },
      { cursor: 1, actor: 1, title: 'Move' },
      { cursor: 2, actor: 1, title: 'End turn', ended_turn: true },
      { cursor: 3, actor: 2, title: 'Raze city' },
      { cursor: 4, actor: 2, title: 'Research' },
      { cursor: 5, actor: 2, title: 'End turn', ended_turn: true },
    ].map((frame) => ({ players: [], tiles: [], age: 1, round: 1, ended_turn: false, ...frame }));
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
        board_history: { id: 'empty-phase-recap', frames },
      },
      seat: 0,
    });
    c.replayLastTurn();
    assert.deepEqual(app.session().playback!.steps, [0, 1, 4]);
    assert.equal(app.session().playback!.end, 4);
    c.stepPlayback(1);
    assert.equal(app.session().playback!.frame!.title, 'Move');
    c.stepPlayback(1);
    assert.equal(app.session().playback!.frame!.title, 'Research');
    c.stepPlayback(-1);
    assert.equal(app.session().playback!.frame!.title, 'Move');
    c.seekPlayback(3);
    assert.equal(
      app.session().playback!.frame!.cursor,
      1,
      'seeking an empty phase shows the preceding action',
    );
    assert.deepEqual(app.sent, []);
  } finally {
    app.close();
  }
});

test('city, tile and unit clicks inspect the same city until Move is opened', async () => {
  const app = paymentController(),
    c = app.controller;
  try {
    const raw = fixture('advances/writing');
    c.setPlayer(0);
    await c.load(engine.stripSecret(raw, 0));
    const unit = app
      .session()
      .view!.units!.find((u) => app.session().view!.cities.some((city) => city.position === u.position))!;
    assert.ok(unit);
    for (const pick of [
      { kind: 'tile' },
      { kind: 'city', player: 0 },
      { kind: 'unit', player: 0, unit: unit.id },
      { kind: 'units', player: 0 },
    ] as const) {
      c.selectTile(unit.position, pick);
      assert.equal(app.session().mode, 'overview');
      assert.equal(app.session().tilePanel, true);
      assert.equal(app.session().focus, unit.position);
      assert.equal(app.session().city, unit.position);
      assert.deepEqual(app.session().selectedUnits, []);
    }
    c.inspectTile(unit.position);
    assert.equal(app.session().tilePanel, true, 'the city dock opens these same tile details');
    c.openSettlers();
    c.selectTile(unit.position, { kind: 'unit', player: 0, unit: unit.id });
    assert.equal(app.session().mode, 'settlers');
    assert.ok(app.session().selectedUnits.includes(unit.id));
    assert.deepEqual(app.sent, [], 'inspection and selection never submit a move');
  } finally {
    app.close();
  }
});
