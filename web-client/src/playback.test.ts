import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recapStart, sinceLastTurn, frameAt, frameEffects, frameDetails } from './playback.ts';
import type { BoardFrame, Game, LoggedAction } from './types.ts';
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
test('finished games skip automatic catch-up while keeping manual last-turn replay available', () => {
  const finished = { ...game, state: 'Finished' };
  assert.equal(recapStart(finished, 0), null);
  assert.equal(recapStart(finished, 0, 11), null);
  assert.deepEqual(sinceLastTurn(finished, 0), { start: 0, end: 3 });
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

test('since last turn includes all intervening players, even after starting your own turn', () => {
  assert.deepEqual(sinceLastTurn(game, 0), { start: 0, end: 3 });
  assert.equal(sinceLastTurn(game, undefined), null);
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
  assert.deepEqual(sinceLastTurn(later, 0), { start: 0, end: 3 });
  assert.deepEqual(sinceLastTurn(later, 1), { start: 3, end: 5 });
  assert.deepEqual(
    sinceLastTurn(
      {
        ...game,
        board_history: {
          id: 'key',
          frames: [...frames, { cursor: 14, actor: 2 }, { cursor: 15, actor: 0 }] as BoardFrame[],
        },
      },
      0,
    ),
    { start: 0, end: 5 },
    'responses do not split an opponent turn',
  );
  assert.deepEqual(
    sinceLastTurn({ ...game, board_history: { id: 'key', frames: frames.slice(1) } }, 0),
    { start: 0, end: 2 },
    'bounded history uses its earliest available position',
  );
  assert.equal(sinceLastTurn({ ...game, board_history: { id: 'key', frames: frames.slice(0, 1) } }, 0), null);
});

test('player 2 round 3 replay includes player 3 round 2 and player 1 round 3', () => {
  const turns = [
    { cursor: 20, actor: 1, ended_turn: true, round: 2 },
    { cursor: 21, actor: 2, round: 2 },
    { cursor: 22, actor: 1, round: 2 }, // Response during player 3's turn.
    { cursor: 23, actor: 2, ended_turn: true, round: 3 },
    { cursor: 24, actor: 0, round: 3 },
    { cursor: 25, actor: 0, ended_turn: true, round: 3 },
    { cursor: 26, actor: 1, round: 3 }, // Current own turn stays out of the replay.
  ] as BoardFrame[];
  const game = { board_history: { id: 'three-players', frames: turns } } as Game;
  assert.deepEqual(sinceLastTurn(game, 1), { start: 0, end: 5 });
  const inProgress = { ...game, board_history: { id: 'three-players', frames: turns.slice(0, 5) } };
  assert.deepEqual(sinceLastTurn(inProgress, 1), { start: 0, end: 4 });
  const earlier = [
    { cursor: 18, actor: 2, ended_turn: true, round: 2 },
    { cursor: 19, actor: 1, round: 2 },
    ...turns,
  ] as BoardFrame[];
  assert.deepEqual(sinceLastTurn({ ...game, board_history: { id: 'three-players', frames: earlier } }, 1), {
    start: 2,
    end: 7,
  });
});

test('since last turn spans four players and an age transition without resetting at the round', () => {
  const frames = [
    { cursor: 1, actor: 2, ended_turn: true, age: 1, round: 3 },
    { cursor: 2, actor: 3, ended_turn: true, age: 1, round: 3 },
    { cursor: 3, actor: null, age: 1, round: 0 },
    { cursor: 4, actor: 2, age: 1, round: 0 },
    { cursor: 5, actor: 0, age: 2, round: 1 },
    { cursor: 6, actor: 0, ended_turn: true, age: 2, round: 1 },
    { cursor: 7, actor: 1, ended_turn: true, age: 2, round: 1 },
  ] as BoardFrame[];
  assert.deepEqual(sinceLastTurn({ board_history: { id: 'four-players', frames } } as Game, 2), {
    start: 0,
    end: 6,
  });
});

test('responses after your attack replay from your last choice, not the defender’s earlier turn', () => {
  // Orderly-dungeon: Maya captures D6, chooses Stelas, then Carthage places
  // Fanaticism infantry and a settler in D7 during Maya's turn.
  const frames = [
    { cursor: 175, actor: 0, ended_turn: true },
    { cursor: 178, actor: 1, title: 'Taxes' },
    { cursor: 179, actor: 1, title: 'Build' },
    { cursor: 181, actor: 1, ended_turn: true },
    { cursor: 182, actor: 0, title: 'Collect' },
    { cursor: 187, actor: 0, title: 'Move' },
    { cursor: 188, actor: 0, title: 'Stelas' },
    { cursor: 189, actor: 1, title: 'Fanaticism' },
    { cursor: 190, actor: 1, title: 'Place Settler' },
  ].map((f) => ({
    players: [],
    tiles: [],
    age: 4,
    round: 3,
    title: '',
    ended_turn: false,
    ...f,
  })) as BoardFrame[];
  const game = { board_history: { id: 'attack-responses', frames } } as Game;
  assert.deepEqual(sinceLastTurn(game, 0), { start: 6, end: 8 });
  assert.equal(recapStart(game, 0), 6, 'returning to your turn catches up on the responses');
  assert.equal(recapStart(game, 0, 189), 7, 'already seen infantry placement stays out');
  assert.equal(recapStart(game, 0, 190), null);
  assert.deepEqual(sinceLastTurn(game, 1), { start: 3, end: 8 }, 'defender still sees the whole attack turn');
  const continued = {
    ...game,
    board_history: { ...game.board_history!, frames: [...frames, { cursor: 191, actor: 0 } as BoardFrame] },
  };
  assert.deepEqual(sinceLastTurn(continued, 0), { start: 6, end: 8 });
  assert.equal(recapStart(continued, 0), null, 'do not interrupt after the attacker has resumed');
  continued.board_history.frames.push({ cursor: 192, actor: 1 } as BoardFrame);
  assert.deepEqual(sinceLastTurn(continued, 0), { start: 9, end: 10 }, 'later responses use the new handoff');
  continued.board_history.frames.push(
    { cursor: 193, actor: 0, ended_turn: true } as BoardFrame,
    { cursor: 194, actor: 1 } as BoardFrame,
  );
  assert.deepEqual(
    sinceLastTurn(continued, 0),
    { start: 11, end: 12 },
    'next ordinary turn uses the usual boundary',
  );
});

test('responses during the first turn need no earlier completed turn', () => {
  const frames = [
    { cursor: 0, actor: null },
    { cursor: 1, actor: 0, title: 'Move' },
    { cursor: 2, actor: 1, title: 'Place Settler' },
  ] as BoardFrame[];
  const game = { board_history: { id: 'first-attack', frames } } as Game;
  assert.deepEqual(sinceLastTurn(game, 0), { start: 1, end: 2 });
  assert.equal(recapStart(game, 0), 1);
  assert.equal(sinceLastTurn(game, undefined), null);
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
  assert.equal(frameDetails(before, after, game).caption, 'Greece · built Academy · gained 2 food, 1 wood');
  assert.equal(
    frameDetails(before, { ...after, title: 'Collect' }, game).caption,
    'Greece · collected 2 food, 1 wood',
  );
});

test('older recaps name the public civilization ability, including free leader actions', () => {
  const before = {
    cursor: 1,
    actor: 0,
    title: 'End turn',
    tiles: [],
    players: [{ id: 1, civilization: 'Carthage', units: [], cities: [] }],
  } as unknown as BoardFrame;
  const after = { ...before, cursor: 2, actor: 1, title: 'Civilization ability' };
  for (const [id, name, start] of [
    ['Hegemony', 'Hegemony', 'Pay 1 action, Start action, Choose a Ship'],
    ['Founder', 'Founder', 'Start action'],
    ['HegemonyFounder', 'Founder', 'Start action in city D2'],
  ]) {
    const action: LoggedAction = {
      action: { Playing: { Custom: { action: id, city: 'D2' } } },
      log: [`Player2: ${name}: ${start}`],
    };
    const game = {
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
                    { ...action, log: ['Player2: Previous ability: Start action'] },
                    action,
                    { ...action, log: ['Player2: Later ability: Start action'] },
                  ],
                },
              ],
            },
          ],
        },
      ],
    } as Game;
    assert.equal(frameDetails(before, after, game).caption, `Carthage · ${name}`);
    assert.equal(
      frameDetails(before, after, { ...game, log_index: 1 }).caption,
      'Carthage · Civilization ability',
      'undone entries are excluded',
    );
    assert.equal(
      frameDetails(before, { ...after, title: 'Navigator' }, game).caption,
      'Carthage · Navigator',
      'new recordings keep their authoritative ability name',
    );
  }
});

test('legacy ability captions fall back safely when the start entry is unavailable', () => {
  const before = { cursor: 0, tiles: [], players: [] } as unknown as BoardFrame;
  const after = { ...before, cursor: 1, title: 'Civilization ability' };
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
                turn_type: { Player: 0 },
                actions: [
                  {
                    action: { Playing: { Custom: { action: 'Hegemony' } } },
                    log: ['Player1: Secret card title: Drew a card'],
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  } as unknown as Game;
  assert.equal(frameDetails(before, after, game).caption, 'Civilization ability');
});
