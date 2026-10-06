import { test } from 'node:test';
import assert from 'node:assert/strict';
import { frameResources, describeResources } from './resource-playback.ts';
import { groupPlaybackFrames, playbackSteps } from './replay-actions.ts';
import { frameDetails, frameEffects, sinceLastTurn, recapStart } from './playback.ts';
import type { Game, LoggedAction } from './types.ts';
const gain = (pile: Record<string, number>, origin = 'Taxes', player = 1) => ({
  player,
  origin: { Ability: origin },
  Resources: { balance: 'Gain', resources: pile },
});
function fixture(actions: LoggedAction[]): Game {
  const positions = ['B1', 'B2', 'C1', 'C2', 'D1'];
  const base = {
    actor: 1,
    ended_turn: false,
    title: 'Taxes',
    age: 1,
    round: 1,
    tiles: positions.map((p) => [p, 'Fertile']),
    players: [
      {
        id: 1,
        civilization: 'Carthage',
        cities: positions.map((position) => ({ position, mood_state: 'Neutral' })),
        units: [],
      },
    ],
  };
  return {
    players: [],
    log_index: actions.length,
    log: [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 1 }, actions }] }] }],
    board_history: {
      id: 'resource-test',
      frames: [
        { ...base, cursor: 0, actor: 0, ended_turn: true },
        ...actions.map((a, i) => ({
          ...base,
          cursor: i + 1,
          actor: a.player ?? 1,
          title: i === 1 ? 'Pay for action' : 'Taxes',
        })),
      ],
    },
  } as unknown as Game;
}
const taxes: LoggedAction[] = [
  { player: 1, action: { Playing: { Custom: { action: 'Taxes' } } } },
  {
    player: 1,
    action: { Response: { Payment: [{ mood_tokens: 1 }] } },
    items: [{ player: 1, Resources: { balance: 'Pay', resources: { mood_tokens: 1 } } }],
  },
  {
    player: 1,
    action: { Response: { ResourceReward: { food: 3, wood: 1, ore: 1 } } },
    items: [gain({ food: 3, wood: 1, ore: 1 })],
  },
];
test('Taxes is one step with the actual mix distributed once over its historical cities', () => {
  const game = fixture(taxes),
    original = JSON.stringify(game),
    frames = groupPlaybackFrames(game);
  assert.deepEqual(
    frames.map((f) => f.cursor),
    [0, 3],
  );
  assert.equal(
    frameDetails(frames[0], frames[1], game).caption,
    'Carthage · Taxes · gained 3 food, 1 wood, 1 ore',
  );
  const { markers } = frameResources(game, 0, frames[1]);
  assert.equal(markers.length, 5);
  assert.equal(new Set(markers.map((m) => m.position)).size, 5);
  assert.deepEqual(
    markers.map((m) => m.resource),
    ['food', 'food', 'food', 'wood', 'ore'],
  );
  game.players = [{ id: 1, civilization: 'Carthage', cities: [{ position: 'Z9', mood_state: 'Happy' }] }];
  assert.deepEqual(frameResources(game, 0, frames[1]).markers, markers);
  game.players = [];
  assert.equal(JSON.stringify(game), original);
  assert.deepEqual(
    frameResources(game, 0, game.board_history!.frames[1]).markers,
    [],
    'payment frames do not reveal later choices',
  );
  game.log_index = 2;
  assert.deepEqual(frameResources(game, 0, frames[1]).gains, [], 'undone outcomes stay excluded');
});
test('collection uses selected tiles, puts bonuses at its city, and subtracts storage overflow', () => {
  const game = fixture([
    {
      player: 1,
      action: {
        Playing: {
          Collect: {
            city_position: 'C2',
            collections: [
              { position: 'B1', pile: { food: 1 }, times: 3 },
              { position: 'B2', pile: { ore: 1 }, times: 2 },
            ],
          },
        },
      },
      items: [
        gain({ food: 3, ore: 1, gold: 1, ideas: 1 }, 'Collect'),
        { player: 1, origin: { Ability: 'Waste' }, Resources: { balance: 'Loss', resources: { food: 2 } } },
      ],
    },
  ]);
  const frame = { ...game.board_history!.frames[1], title: 'Collect' };
  const result = frameResources(game, 0, frame);
  assert.equal(describeResources(result.gains[0].pile), '1 food, 1 ore, 1 idea, 1 gold');
  assert.deepEqual(
    result.markers.map((m) => [m.position, m.resource, m.amount]),
    [
      ['B1', 'food', 1],
      ['B2', 'ore', 1],
      ['C2', 'ideas', 1],
      ['C2', 'gold', 1],
    ],
  );
  assert.match(frameDetails(game.board_history!.frames[0], frame, game).caption, /2 food not stored/);
});
test('recruiting, construction, research, happiness and card follow-ups group without swallowing separate actions', () => {
  for (const name of ['Recruit', 'Construct', 'Advance', 'IncreaseHappiness', 'ActionCard', 'WonderCard']) {
    const game = fixture([
      { player: 1, action: { Playing: { [name]: { city_position: 'C2' } } } },
      taxes[1],
      {
        player: 1,
        action: { Response: { ResourceReward: { wood: 1 } } },
        items: [gain({ wood: 1 }, 'Medicine')],
      },
      { player: 1, action: { Playing: { Collect: { city_position: 'B1', collections: [] } } } },
    ]);
    assert.deepEqual(
      groupPlaybackFrames(game).map((f) => f.cursor),
      [0, 3, 4],
      name,
    );
  }
});
test('battles, incident triggers, opponent reactions and objective claims keep separate replay steps', () => {
  const boundaries: LoggedAction[] = [
    { player: 0, action: { Response: { Bool: true } }, items: [gain({ food: 1 }, 'Medicine', 0)] },
    {
      player: 1,
      action: { Response: { Bool: true } },
      items: [{ player: 1, Text: 'triggers the event Earthquake' }],
    },
    { player: 1, action: { Response: { Bool: true } }, combat_stats: {} },
    {
      player: 1,
      action: { Response: { Bool: true } },
      items: [{ player: 1, HandCard: { to: { CompleteObjective: 'Bold' } } }],
    },
  ];
  for (const boundary of boundaries) {
    const game = fixture([taxes[0], boundary, taxes[2]]);
    assert.deepEqual(
      groupPlaybackFrames(game).map((f) => f.cursor),
      [0, 1, 2, 3],
    );
  }
});
test('merged card-draw cues retain their cursor and never repeat on the next response', () => {
  const game = fixture(taxes);
  game.board_history!.frames[1].effects = [{ player: 1, kind: 'action', label: 'Drew a card' }];
  game.board_history!.frames[3].effects = [{ player: 1, kind: 'objective', label: 'Drew an objective' }];
  game.board_history!.frames = groupPlaybackFrames(game);
  assert.deepEqual(
    frameEffects(game, 1).map((e) => e.kind),
    ['objective'],
  );
  assert.deepEqual(
    frameEffects(game, 0).map((e) => e.kind),
    ['action', 'objective'],
  );
});

test('a prior event trigger cannot anchor later event rewards to the initiating collection city', () => {
  const game = fixture([
    {
      player: 1,
      action: { Playing: { Collect: { city_position: 'C2', collections: [] } } },
      items: [{ player: 1, Text: 'triggers the event Earthquake' }],
    },
    { player: 1, action: { Response: { Bool: true } }, items: [gain({ food: 1 }, 'Reward')] },
  ]);
  const result = frameResources(game, 1, game.board_history!.frames[2]);
  assert.equal(describeResources(result.gains[0].pile), '1 food');
  assert.deepEqual(result.markers, []);
});

test('exploration orientation and ship landing are one move; unrelated movement responses stay separate', () => {
  const game = fixture([
    { player: 1, action: { Movement: { Move: { units: [1], destination: 'B1' } } } },
    { player: 1, action: { Response: { ExploreResolution: 3 } } },
    { player: 1, action: { Response: { SelectPositions: ['B2'] } } },
    { player: 1, action: { Response: { SelectPositions: ['C2'] } } },
  ]);
  const frames = game.board_history!.frames;
  frames[1].title = 'Move';
  frames[2].title = 'Explore Resolution';
  frames[3].title = 'Finish ship exploration';
  frames[4].title = 'Place Settler';
  const grouped = groupPlaybackFrames(game);
  assert.deepEqual(
    grouped.map((f) => f.cursor),
    [0, 3, 4],
  );
  assert.equal(grouped[1].title, 'Explore');
  assert.equal(grouped[2].title, 'Place Settler');
  assert.equal(frameDetails(frames[1], frames[2], game).caption, 'Carthage · Explore');
});

test('declining to raze creates no playback step or replay button, while razing stays visible', () => {
  const game = fixture([
    {
      player: 1,
      action: { Response: { SelectPositions: [] } },
      items: [{ player: 1, Text: 'Did not raze a city' }],
    },
  ]);
  game.board_history!.frames[1].title = 'Raze city';
  const grouped = groupPlaybackFrames(game);
  assert.equal(grouped.length, 1);
  const clean = { ...game, board_history: { ...game.board_history!, frames: grouped } };
  assert.equal(sinceLastTurn(clean, 0), null);
  assert.equal(recapStart(clean, 0), null);
  game.board_history!.frames[1].effects = [{ player: 1, kind: 'action', label: 'Gain 1 gold' }];
  assert.equal(groupPlaybackFrames(game).length, 2, 'do not hide rewards or other consequences');
  game.board_history!.frames[1].effects = [];
  (game.log![0].rounds[0].turns[0].actions![0].action as any).Response.SelectPositions = ['B1'];
  assert.equal(groupPlaybackFrames(game).length, 2);
});
test('ending an otherwise empty opponent turn does not offer replay', () => {
  const game = fixture([{ player: 1, action: { Playing: 'EndTurn' } }]);
  game.board_history!.frames[1].title = 'End turn';
  game.board_history!.frames[1].ended_turn = true;
  assert.equal(sinceLastTurn(game, 0), null);
  assert.equal(recapStart(game, 0), null);
});

test('empty phase decisions and movement stops are skipped without losing turn boundaries', () => {
  const game = fixture([
    { player: 1, action: { Playing: { Custom: { action: 'Taxes' } } }, items: [gain({ food: 1 })] },
    { player: 1, action: { Response: { SelectPositions: [] } } },
    { player: 1, action: { Response: { Bool: false } } },
    { player: 1, action: { Movement: 'Stop' } },
    { player: 1, action: { Playing: 'EndTurn' } },
  ]);
  const frames = game.board_history!.frames;
  frames[2].title = 'Raze city';
  frames[3].title = 'Resolve choice';
  frames[4].title = 'Stop movement';
  frames[5].title = 'End turn';
  frames[5].ended_turn = true;
  frames[5].players = structuredClone(frames[5].players);
  frames[5].players[0].cities![0].activations = 0;
  const grouped = groupPlaybackFrames(game);
  assert.deepEqual(
    grouped.map((frame) => frame.cursor),
    [0, 1, 5],
  );
  const clean = { ...game, board_history: { ...game.board_history!, frames: grouped } };
  assert.deepEqual(sinceLastTurn(clean, 0), { start: 0, end: 2 });
  assert.deepEqual(playbackSteps(clean, 0, 2), [0, 1]);
  assert.equal(grouped[2].ended_turn, true);
});

test('phase transitions retain actual public rewards, card draws and city changes', () => {
  const game = fixture([
    { player: 1, action: { Playing: 'EndTurn' }, items: [gain({ food: 1 }, 'TradeRoutes')] },
    { player: 1, action: { Response: { SelectPositions: [] } } },
    { player: 1, action: { Response: { Bool: false } } },
  ]);
  const frames = game.board_history!.frames;
  frames[1].title = 'End turn';
  frames[1].ended_turn = true;
  frames[2].title = 'Raze city';
  frames[2].effects = [{ player: 1, kind: 'objective', label: 'Drew an objective' }];
  frames[3].title = 'Resolve choice';
  frames[3].players = structuredClone(frames[3].players);
  frames[3].players[0].cities!.pop();
  assert.deepEqual(playbackSteps(game, 0, 3), [0, 1, 2, 3]);
});

test('phase confirmation and age bookkeeping without consequences do not offer replay', () => {
  const game = fixture([{ player: 1, action: { Response: { Bool: true } }, items: [gain({ food: 0 })] }]);
  const frames = game.board_history!.frames;
  frames[1].title = 'Resolve choice';
  frames[1].age = 2;
  frames[1].round = 1;
  frames[1].players = structuredClone(frames[1].players);
  frames[1].players[0].cities![0].activations = 0;
  assert.deepEqual(playbackSteps(game, 0, 1), [0]);
  assert.equal(sinceLastTurn(game, 0), null);
  assert.equal(recapStart(game, 0), null);
});
