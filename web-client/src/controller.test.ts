import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { build } from 'esbuild';
import type { Controller as ControllerType } from './controller';
import type { Session } from './types';

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
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
  );
  const off = controller.session.subscribe((s) => {
    session = s;
  });
  return {
    controller,
    sent,
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
