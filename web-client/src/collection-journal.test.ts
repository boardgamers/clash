import { test } from 'node:test';
import assert from 'node:assert/strict';
import { journal } from './journal.ts';
import type { Game, LoggedAction, Pile } from './types.ts';

function game(actions: LoggedAction[]): Game {
  return {
    state: 'Playing',
    players: [{ id: 0, name: 'Player1', civilization: 'Greece' }],
    map: { tiles: [] },
    current_player_index: 0,
    actions_left: 1,
    age: 1,
    round: 1,
    log_index: actions.length,
    log: [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 0 }, actions }] }] }],
  };
}
const setup: LoggedAction = {
  log: ['Player1: Setup: Play as Greece, Gain city D7, City D7 became Happy'],
  items: [
    {
      player: 0,
      origin: { Ability: 'Setup' },
      Structure: { structure: 'CityCenter', position: 'D7', balance: 'Gain' },
    },
    { player: 0, MoodChange: { city: 'D7', mood: 'Happy' } },
  ],
};
const academy: LoggedAction = {
  log: ['Player1: Construct: Gain Academy at D7'],
  items: [{ player: 0, Structure: { structure: { Building: 'Academy' }, position: 'D7', balance: 'Gain' } }],
};
function collect(piles: Pile[], gain: string, sources: string[] = [], times = 1): LoggedAction {
  return {
    action: {
      Playing: {
        Collect: {
          city_position: 'D7',
          collections: piles.map((pile, i) => ({ position: `C${i + 1}`, pile, times })),
        },
      },
    },
    log: [
      `Player1: Collect: Pay 1 action, Use city D7, Gain ${gain}`,
      ...sources.map((s) => `Player1: ${s}`),
    ],
  };
}

test('collection explains historical city size and pre-activation mood, with education already in the total', () => {
  const first = collect([{ food: 1 }, { food: 1 }, { wood: 1 }], '2 food, 1 wood and 1 idea', [
    'Public Education: Gain 1 idea',
  ]);
  first.items = [{ player: 0, MoodChange: { city: 'D7', mood: 'Neutral' } }];
  const state = game([setup, academy, first, collect([{ ore: 1 }, { ore: 1 }], '2 ore')]);
  state.players[0].cities = [
    { position: 'D7', mood_state: 'Angry', city_pieces: { academy: 0, fortress: 0 } },
  ];
  const entries = journal(state).filter((e) => e.collection);
  assert.equal(entries.length, 2);
  assert.deepEqual(entries[0].collection!.city, {
    size: 2,
    mood: 'Happy',
    structures: ['Settlement', 'Academy'],
  });
  assert.equal(entries[1].collection!.city!.mood, 'Neutral');
  assert.deepEqual(
    entries[0].collection!.effects.map((e) => e.source),
    ['Public Education'],
  );
  assert.equal(entries[0].tokens.find((t) => t.icon === 'ideas')?.value, '+1');
  assert.equal(
    entries[1].collection!.effects.length,
    0,
    'A later collection does not repeat a once-per-turn bonus',
  );
  assert.ok(!journal(state).some((e) => e.title === 'Public Education'), 'No duplicate gain row');
});

test('rice additions and metallurgy conversion reconcile with the collection total', () => {
  const entries = journal(
    game([
      collect([{ food: 1 }], '2 food', ['Rice Cultivation: Added 1 food']),
      collect([{ ore: 1 }, { ore: 1 }], '1 ore and 1 gold', ['Metallurgy: Convert 1 ore to 1 gold']),
    ]),
  );
  assert.equal(entries.length, 2);
  assert.equal(entries[0].collection!.effects[0].source, 'Rice Cultivation');
  assert.deepEqual(
    entries[1].collection!.effects[0].tokens.map((t) => [t.icon, t.value]),
    [
      ['ore', '−1'],
      ['gold', '+1'],
    ],
  );
  assert.equal(entries[0].collection!.city, undefined, 'Incomplete history must not invent a city size');
});

test('unreconciled effects stay separate, repeated tile selections count, and storage losses remain', () => {
  const mismatch = journal(game([collect([{ food: 1 }], '1 food', ['Public Education: Gain 1 idea'])]));
  assert.equal(mismatch.length, 2);
  assert.deepEqual(mismatch[0].collection!.effects, []);
  const repeated = collect([{ food: 1 }], '3 food and 1 idea', ['Public Education: Gain 1 idea'], 3);
  repeated.log!.push('Player1: Collect: Could not store 2 food');
  const entries = journal(game([repeated]));
  assert.equal(entries[0].collection!.effects[0].source, 'Public Education');
  assert.equal(entries[0].collection!.tiles[0].times, 3);
  assert.ok(entries.some((e) => e.text.includes('Could not store 2 food')));
});

test('Canals explains the extra food without duplicating the gain', () => {
  const entries = journal(
    game([collect([{ food: 1 }, { wood: 1 }], '2 food and 1 wood', ['Canals: Added 1 food'])]),
  );
  assert.equal(entries.length, 1);
  assert.equal(entries[0].tokens.find((t) => t.icon === 'food')?.value, '+2');
  assert.equal(entries[0].collection!.effects[0].source, 'Canals');
});

test('undone collection commands have no visible outcome', () => {
  const undone = collect([{ food: 1 }], '1 food');
  undone.log = [];
  assert.equal(journal(game([setup, undone])).filter((e) => e.collection).length, 0);
});

test('resource totals remain analytical when a collection range modifier is logged', () => {
  const [entry] = journal(
    game([collect([{ food: 1 }], '2 food with Irrigation', ['Rice Cultivation: Added 1 food'])]),
  );
  assert.equal(entry.tokens.find((t) => t.icon === 'food')?.value, '+2');
  assert.deepEqual(entry.notes, ['With Irrigation']);
  assert.equal(entry.collection!.effects[0].source, 'Rice Cultivation');
});
