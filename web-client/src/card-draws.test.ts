import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { CardDrawTracker } from './card-draws.ts';
import type { View, Game } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('card reveals detect a confirmed draw once, stay silent on initial load or seat changes, and ignore undo returns', async () => {
  let raw = await engine.init(
    2,
    [],
    { civilization: 'Random' },
    'clash-preview-20260927',
    {},
  );
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
