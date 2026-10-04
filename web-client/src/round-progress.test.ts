import { test } from 'node:test';
import assert from 'node:assert/strict';
import { roundProgress, personalRoundLabel } from './round-progress.ts';
import type { Game, PlayerView } from './types.ts';

const players = [0, 1, 2, 3].map((index) => ({ index }) as PlayerView);
const game = (overrides: Partial<Game> = {}) =>
  ({
    state: 'Playing',
    round: 3,
    starting_player_index: 2,
    current_player_index: 0,
    ...overrides,
  }) as Game;

test('turn order wraps from the starting player and shows which turns in this round are complete', () => {
  const progress = roundProgress(game(), players);
  assert.deepEqual(
    progress.map((p) => [p.player.index, p.order, p.status]),
    [
      [2, 1, 'done'],
      [3, 2, 'done'],
      [0, 3, 'current'],
      [1, 4, 'upcoming'],
    ],
  );
  assert.equal(personalRoundLabel(3, progress[0].status), 'Final turn complete');
  assert.equal(personalRoundLabel(3, progress[2].status), 'Your final turn');
  assert.equal(personalRoundLabel(3, progress[3].status), 'Final turn ahead');
});

test('an opponent responding during combat does not change whose turn is in progress', () => {
  const g = game({ events: [{ event_type: { Combat: { defender: 2 } } }] });
  const progress = roundProgress(g, players);
  assert.equal(progress.find((p) => p.player.index === 0)?.status, 'current');
  assert.equal(progress.find((p) => p.player.index === 2)?.status, 'done');
});

test('the next round and a new starting player reset round progress', () => {
  assert.deepEqual(
    roundProgress(game({ round: 1, starting_player_index: 1, current_player_index: 1 }), players).map(
      (p) => p.status,
    ),
    ['current', 'upcoming', 'upcoming', 'upcoming'],
  );
  assert.equal(personalRoundLabel(1, 'upcoming'), 'Turn ahead');
  assert.equal(personalRoundLabel(2, 'done'), 'Turn complete');
  assert.ok(roundProgress(game({ round: 4 }), players).every((p) => p.status === 'done'));
  assert.equal(personalRoundLabel(4, 'done'), null);
});

test('dropped players have no turn number; draft, replay and finished games have no live progress', () => {
  assert.deepEqual(
    roundProgress(game({ dropped_players: [3] }), players).map((p) => [p.player.index, p.order, p.status]),
    [
      [2, 1, 'done'],
      [0, 2, 'current'],
      [1, 3, 'upcoming'],
      [3, undefined, 'left'],
    ],
  );
  for (const g of [null, game({ state: 'Finished' }), game({ state: 'ChooseCivilization' })])
    assert.ok(roundProgress(g, players).every((p) => !p.order && !p.status));
});
