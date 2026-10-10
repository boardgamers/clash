import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { CardDrawTracker } from './card-draws.ts';
import type { View, Game } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('card reveals detect a confirmed draw once, stay silent on initial load or seat changes, and ignore undo returns', async () => {
  let raw = await engine.init(2, [], { civilization: 'Random' }, 'clash-preview-20260927', {});
  const seat = engine.currentPlayer(raw);
  const tracker = new CardDrawTracker();
  const snapshot = () => {
    const game: Game = JSON.parse(engine.stripSecret(raw, seat));
    const view: View = JSON.parse(engine.webView(JSON.stringify(game), seat));
    return { game, view };
  };
  const before = snapshot();
  assert.deepEqual(tracker.update(seat, before.game, before.view), []);
  raw = engine.tryMove(
    raw,
    JSON.stringify(before.view.advances.find((a) => a.id === 'Engineering')!.action),
    seat,
  );
  const after = snapshot();
  const drawn = tracker.update(seat, after.game, after.view);
  assert.equal(drawn.length, 1);
  assert.equal(drawn[0].kind, 'wonder');
  assert.deepEqual(
    tracker.update(seat, after.game, after.view),
    [],
    'A duplicate or reconnect state cannot repeat the reveal',
  );
  tracker.reset();
  assert.deepEqual(
    tracker.update(seat, after.game, after.view),
    [],
    'An existing card on load is not a draw',
  );
  assert.deepEqual(tracker.update(1 - seat, after.game, after.view), [], 'Switching seats is not a draw');

  // An objective returned by undo becomes visible again, but history moved backwards.
  const discarded = structuredClone(after.view);
  discarded.objectiveCards = [];
  const later = { ...after.game, log_index: after.game.log_index + 1 };
  tracker.reset();
  tracker.update(seat, later, discarded);
  assert.deepEqual(tracker.update(seat, after.game, after.view), []);
});

test('Writing reveals the drawn action and objective cards once with full card details', async () => {
  let raw = await engine.init(2, [], { civilization: 'Random' }, 'clash-preview-20260927', {});
  const seat = engine.currentPlayer(raw),
    tracker = new CardDrawTracker();
  const snapshot = () => {
    const game: Game = JSON.parse(engine.stripSecret(raw, seat));
    return { game, view: JSON.parse(engine.webView(JSON.stringify(game), seat)) as View };
  };
  const before = snapshot();
  tracker.update(seat, before.game, before.view);
  raw = engine.tryMove(
    raw,
    JSON.stringify(before.view.advances.find((a) => a.id === 'Writing')!.action),
    seat,
  );
  const after = snapshot(),
    drawn = tracker.update(seat, after.game, after.view);
  assert.deepEqual(
    drawn.map((draw) => draw.kind),
    ['action', 'objective'],
  );
  const action = drawn.find((draw) => draw.kind === 'action')!;
  assert.ok(action.card.name);
  assert.ok(action.card.description);
  assert.ok(action.card.tactics?.description);
  assert.deepEqual(tracker.update(seat, after.game, after.view), []);
  const withoutCard = structuredClone(after.view);
  withoutCard.actionCards = withoutCard.actionCards!.filter((card) => card.id !== action.card.id);
  tracker.update(seat, { ...after.game, log_index: after.game.log_index + 1 }, withoutCard);
  assert.deepEqual(
    tracker.update(seat, after.game, after.view),
    [],
    'undo must not reveal returned action cards',
  );
});

test('end-of-age draws reveal action and objective cards for the viewing player', async () => {
  const initial = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/status_phase/free_advance.json', import.meta.url),
      'utf8',
    ),
  );
  initial.players = [
    ...initial.players.slice(0, 2),
    ...JSON.parse(await engine.init(2, [], {}, 'draw-npcs', {})).players.slice(2),
  ];
  let raw = JSON.stringify(initial);
  const tracker = new CardDrawTracker();
  const snapshot = () => {
    const game: Game = JSON.parse(engine.stripSecret(raw, 0));
    return { game, view: JSON.parse(engine.webView(JSON.stringify(game), 0)) as View };
  };
  const before = snapshot();
  tracker.update(0, before.game, before.view);
  for (const action of [
    { Playing: 'EndTurn' },
    { Response: { SelectAdvance: 'Storage' } },
    { Response: { SelectAdvance: 'Philosophy' } },
  ]) {
    raw = engine.tryMove(raw, JSON.stringify(action), engine.currentPlayer(raw));
  }
  const after = snapshot(),
    drawn = tracker.update(0, after.game, after.view);
  assert.deepEqual(drawn.map((draw) => draw.kind).sort(), ['action', 'objective']);
});
