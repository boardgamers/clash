import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recapStart, frameAt, frameEffects } from './playback.ts';
import type { BoardFrame, Game } from './types.ts';
const frames = [
  { cursor: 10, actor: 0, ended_turn: true },
  { cursor: 11, actor: 1 },
  { cursor: 12, actor: 1, effects: [{ player: 1, kind: 'action', label: 'Drew an action card' }] },
  { cursor: 13, actor: 1, ended_turn: true },
] as BoardFrame[];
const game = { board_history: { id: 'public-key', frames } } as Game;
test('catch-up resumes only unseen opponent actions after the viewer ended their turn', () => {
  assert.equal(recapStart(game, 0), 0);
  assert.equal(recapStart(game, 0, 11), 1);
  assert.equal(recapStart(game, 0, 13), null);
  assert.equal(recapStart(game, undefined), null);
  assert.equal(recapStart(game, 1), null);
  assert.equal(
    recapStart(
      { ...game, board_history: { id: 'key', frames: [...frames, { cursor: 14, actor: 0 } as BoardFrame] } },
      0,
    ),
    null,
  );
});
test('replay cursors clamp to the recorded range; effects identify card types only', () => {
  assert.equal(frameAt(frames, 0), 0);
  assert.equal(frameAt(frames, 100), 3);
  assert.equal(frameAt(frames, 12), 2);
  assert.deepEqual(frameEffects(game, 11), [
    { player: 1, kind: 'action', label: 'Drew an action card', key: 'public-key:12:0' },
  ]);
  assert.deepEqual(frameEffects(game, 13), []);
});
