import { test } from 'node:test';
import assert from 'node:assert/strict';
import { liveActionMessages } from './live-actions.ts';
import type { Game } from './types.ts';

function state(count: number): Game {
  const players = [{ id: 1, civilization: 'Greece', cities: [], units: [] }];
  const actions = ['Philosophy', 'Arts'].slice(0, count).map((advance) => ({
    player: 1, action: { Playing: { Advance: { advance } } },
    items: [{ player: 1, Advance: { advance, balance: 'Gain' } }],
  }));
  const base = { actor: 1, age: 1, round: 1, tiles: [], players, ended_turn: false, title: 'Research' };
  return {
    players, log_index: count,
    log: [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 1 }, actions }] }] }],
    board_history: { id: 'live', frames: Array.from({ length: count + 1 }, (_, cursor) => ({ ...base, cursor })) },
  } as unknown as Game;
}

test('live opponent and spectator notices name researched advances and retain multiple actions', () => {
  assert.deepEqual(liveActionMessages(state(0), state(2), 0), [
    'Greece · researched Philosophy', 'Greece · researched Arts',
  ]);
  assert.deepEqual(liveActionMessages(state(0), state(1)), ['Greece · researched Philosophy']);
});

test('live notices exclude own actions, repeated snapshots, undo, and replaced histories', () => {
  assert.deepEqual(liveActionMessages(state(0), state(1), 1), []);
  assert.deepEqual(liveActionMessages(state(1), state(1), 0), []);
  assert.deepEqual(liveActionMessages(state(2), state(1), 0), []);
  const replaced = state(2);
  replaced.board_history!.id = 'replayed';
  assert.deepEqual(liveActionMessages(state(1), replaced, 0), []);
  assert.deepEqual(liveActionMessages(state(1), state(2), 0), ['Greece · researched Arts']);
});
