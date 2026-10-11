import { test } from 'node:test';
import assert from 'node:assert/strict';
import { UndoPreview } from './undo-preview.ts';
import type { Game, View } from './types.ts';

test('Undo keeps earlier snapshots when host action object fields are reordered', () => {
  const cache = new UndoPreview();
  const before = {
    board_history: { id: 'undo-order', frames: [] },
    current_player_index: 0,
    age: 1,
    round: 1,
    log_index: 1,
    log: [
      {
        age: 1,
        rounds: [
          {
            round: 1,
            turns: [
              {
                turn_type: { Player: 0 },
                actions: [
                  { action: { Playing: { Custom: { action: 'Theaters', payment: { food: 1, gold: 2 } } } } },
                ],
              },
            ],
          },
        ],
      },
    ],
  } as unknown as Game;
  const view = { canUndo: true } as View;
  cache.remember('earlier snapshot', before, view, 0);
  const after = structuredClone(before);
  after.log_index = 2;
  const actions = after.log![0].rounds[0].turns[0].actions!;
  actions[0].action = { Playing: { Custom: { payment: { gold: 2, food: 1 }, action: 'Theaters' } } };
  actions.push({ action: { Response: { ResourceReward: { gold: 1 } } } });
  cache.remember('later snapshot', after, view, 0);
  assert.equal(cache.previous(after, view), 'earlier snapshot');
  actions[0].action = { Playing: { Custom: { action: 'Sports' } } };
  assert.equal(cache.previous(after, view), undefined, 'A genuinely different branch cannot reuse Undo');
});
