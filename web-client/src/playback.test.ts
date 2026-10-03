import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recapStart, lastOpponentTurn, frameAt, frameEffects, frameDetails } from './playback.ts';
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

test('last turn selects the latest opponent turn, even after returning or starting your own turn', () => {
  assert.deepEqual(lastOpponentTurn(game, 0), { start: 0, end: 3 });
  assert.equal(lastOpponentTurn(game, undefined), null);
  const later = {
    ...game,
    board_history: {
      id: 'key',
      frames: [
        ...frames,
        { cursor: 14, actor: 0 },
        { cursor: 15, actor: 0, ended_turn: true },
      ] as BoardFrame[],
    },
  };
  assert.deepEqual(lastOpponentTurn(later, 0), { start: 0, end: 3 });
  assert.deepEqual(lastOpponentTurn(later, 1), { start: 3, end: 5 });
  assert.deepEqual(
    lastOpponentTurn(
      {
        ...game,
        board_history: {
          id: 'key',
          frames: [...frames, { cursor: 14, actor: 2 }, { cursor: 15, actor: 0 }] as BoardFrame[],
        },
      },
      0,
    ),
    { start: 3, end: 5 },
    'responses do not split an opponent turn',
  );
  assert.deepEqual(
    lastOpponentTurn({ ...game, board_history: { id: 'key', frames: frames.slice(1) } }, 0),
    { start: 0, end: 2 },
    'bounded history uses its earliest available position',
  );
  assert.equal(
    lastOpponentTurn({ ...game, board_history: { id: 'key', frames: frames.slice(0, 1) } }, 0),
    null,
  );
});

test('recap captions and highlights describe public recruitment and movement without coordinates in text', () => {
  const before = {
    cursor: 1,
    title: 'End turn',
    actor: 0,
    tiles: [['A1', 'Fertile']],
    players: [{ id: 1, civilization: 'Greece', units: [], cities: [] }],
  } as unknown as BoardFrame;
  const after = {
    ...before,
    cursor: 2,
    actor: 1,
    title: 'Recruit',
    players: [
      {
        ...before.players[0],
        units: [
          { id: 1, unit_type: 'Infantry', position: 'A1' },
          { id: 2, unit_type: 'Infantry', position: 'A1' },
        ],
      },
    ],
  } as BoardFrame;
  assert.deepEqual(frameDetails(before, after), {
    caption: 'Greece · recruited 2 infantry',
    positions: ['A1'],
  });
  const moved = structuredClone(after);
  moved.title = 'Move';
  moved.players[0].units![0].position = 'A2';
  assert.deepEqual(frameDetails(after, moved), { caption: 'Greece · Move', positions: ['A2', 'A1'] });
  const unchanged = structuredClone(moved);
  unchanged.title = 'Research';
  assert.deepEqual(frameDetails(moved, unchanged), { caption: 'Greece · Research', positions: [] });
});

test('recap names researched advances from its public log interval, including old recordings', () => {
  const before = {
    cursor: 1,
    actor: 0,
    title: 'End turn',
    tiles: [],
    players: [{ id: 1, civilization: 'Greece', units: [], cities: [] }],
  } as unknown as BoardFrame;
  const after = { ...before, cursor: 2, actor: 1, title: 'Research' };
  const history = {
    log_index: 3,
    log: [
      {
        age: 1,
        rounds: [
          {
            round: 1,
            turns: [
              {
                turn_type: { Player: 1 },
                actions: [
                  { items: [{ player: 1, Advance: { advance: 'Navigation', balance: 'Gain' } }] },
                  {
                    items: [
                      { player: 1, Advance: { advance: 'SteelWeapons', balance: 'Gain' } },
                      { player: 1, Advance: { advance: 'Dogma', balance: 'Loss' } },
                    ],
                  },
                  { items: [{ player: 1, Advance: { advance: 'Metallurgy', balance: 'Gain' } }] },
                ],
              },
            ],
          },
        ],
      },
    ],
  } as Game;
  assert.equal(frameDetails(before, after, history).caption, 'Greece · researched Steel Weapons');
  assert.equal(
    frameDetails(before, after, { ...history, log_index: 1 }).caption,
    'Greece · Research',
    'undone research is not presented',
  );
});

test('recap details name construction and collected amounts without exposing coordinates or cards', () => {
  const before = {
    cursor: 0,
    actor: 0,
    title: 'End turn',
    tiles: [],
    players: [{ id: 1, civilization: 'Greece', units: [], cities: [] }],
  } as unknown as BoardFrame;
  const game = {
    log_index: 1,
    log: [
      {
        age: 1,
        rounds: [
          {
            round: 1,
            turns: [
              {
                turn_type: { Player: 1 },
                actions: [
                  {
                    items: [
                      {
                        player: 1,
                        Structure: { structure: { Building: 'Academy' }, balance: 'Gain', position: 'A1' },
                      },
                      { player: 1, Resources: { resources: { food: 2, wood: 1 }, balance: 'Gain' } },
                      { player: 1, HandCard: { to: { Hand: 1 }, card: { ActionCard: 123 } } },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  } as Game;
  const after = { ...before, cursor: 1, actor: 1, title: 'Build' };
  assert.equal(frameDetails(before, after, game).caption, 'Greece · built Academy');
  assert.equal(
    frameDetails(before, { ...after, title: 'Collect' }, game).caption,
    'Greece · collected 2 food, 1 wood',
  );
});
