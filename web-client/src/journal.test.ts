import { test } from 'node:test';
import assert from 'node:assert/strict';
import { journal, journalParts } from './model.ts';
import type { Game, LoggedAction } from './types.ts';

test('map references preserve surrounding text and avoid matching names or resource amounts', () => {
  const text = 'Move from D2 to E8; gain 3 food. Player1 has 2 ideas.';
  const parts = journalParts(text);
  assert.equal(parts.map((p) => p.text).join(''), text);
  assert.deepEqual(
    parts.filter((p) => p.position).map((p) => p.position),
    ['D2', 'E8'],
  );
  assert.deepEqual(
    parts.filter((p) => p.resource).map((p) => p.resource),
    ['food', 'ideas'],
  );
});

test('display names matching game terms do not overwrite research or resources', () => {
  const state = game([
    {
      log: ['Player1: Advance: Pay 2 food, Gain Engineering and take an event token (2 left)'],
    },
  ]);
  state.players[0].name = 'Engineering';
  state.players[1].name = 'food';
  const [entry] = journal(state);
  assert.equal(entry.tokens.find((t) => t.icon === 'research')?.label, 'Engineering');
  assert.equal(entry.tokens.find((t) => t.icon === 'food')?.value, '−2');
});

function game(actions: LoggedAction[]): Game {
  return {
    state: 'Playing',
    players: [
      { id: 0, name: 'Leif', civilization: 'Vikings' },
      { id: 1, name: 'Aurelia', civilization: 'Rome' },
    ],
    map: { tiles: [] },
    current_player_index: 0,
    actions_left: 1,
    age: 1,
    round: 1,
    log_index: 0,
    log: [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 0 }, actions }] }] }],
  };
}

test('naval rolls omit unused army die symbols while preserving tactics, hits and ship losses', () => {
  const state = game([
    {
      combat_stats: { battleground: 'Sea' },
      log: [
        'Leif: Combat: Roll 3 (infantry, no bonus) for combined combat value of 5 and gets 1 hits against defending units, Combat modifiers: High Morale added 2 combat value',
        'Pirates: Combat: Roll 4 (elephant, no bonus) for combined combat value of 4 and gets 0 hits against attacking units, Lost 1 ship at D3',
      ],
    },
  ]);
  state.players.push({ id: 3, civilization: 'Pirates' });
  const [attacker, defender] = journal(state);
  assert.deepEqual(attacker.notes, [
    'Roll 3 → combat value 5 → 1 hit against defending units',
    'Combat modifiers: High Morale added 2 combat value',
  ]);
  assert.deepEqual(defender.notes, ['Roll 4 → combat value 4 → 0 hits against attacking units']);
  assert.deepEqual(
    defender.tokens.map((t) => [t.value, t.label]),
    [['−1', 'ship at D3']],
  );
});

test('unfinished naval battles also omit inactive symbols on each die', () => {
  // Combat stats are only stored when the entire battle ends.
  const [entry] = journal(
    game([
      {
        log: [
          'Leif: Combat: Attacking with 2 ships, Roll 4 (elephant, no bonus), 1 (leader, no bonus) for combined combat value of 5 and gets 1 hits against defending units',
        ],
      },
    ]),
  );
  assert.deepEqual(entry.notes, [
    'Attacking with 2 ships',
    'Roll 4, 1 → combat value 5 → 1 hit against defending units',
  ]);
});

test('land rolls identify activated die symbols and preserve rerolls, bonuses and cancellations', () => {
  const [entry] = journal(
    game([
      {
        log: [
          'Leif: Combat: Attacking with 1 infantry, 1 elephant and 1 leader, Roll 1 (leader, re-roll), 6 (infantry, +1 combat value), 4 (elephant, -1 hits, no combat value), 5 (cavalry, no bonus) for combined combat value of 12 and gets 2 hits against defending units',
        ],
      },
    ]),
  );
  assert.equal(
    entry.notes.join(', '),
    'Attacking with 1 infantry, 1 elephant and 1 leader, Roll 1 (leader die symbol: re-roll), 6 (infantry die symbol: +1 combat value), 4 (elephant die symbol: -1 hits, no combat value), 5 → combat value 12 → 2 hits against defending units',
  );
});

test('combat wording preserves no-dice rolls and unknown rule details', () => {
  const [entry] = journal(
    game([
      {
        log: [
          'Leif: Combat: Roll no dice for combined combat value of 0 and gets 0 hits against defending units, Custom combat effect',
        ],
      },
    ]),
  );
  assert.deepEqual(entry.notes, [
    'Roll no dice → combat value 0 → 0 hits against defending units',
    'Custom combat effect',
  ]);
});

test('journal combines research and its card draw without revealing hidden card identities', () => {
  const entries = journal(
    game([
      {
        action: { Playing: { Advance: { advance: 'Engineering' } } },
        log: [
          'Leif: Advance: Pay 1 action, Pay 2 food, Gain Engineering and take an event token (2 left)',
          'Leif: Engineering: Draw a wonder card',
        ],
        items: [{ player: 0, HandCard: { to: { Hand: 0 } }, origin: { Advance: 'Engineering' } }],
      },
    ]),
  );
  assert.equal(entries.length, 1);
  assert.equal(entries[0].civilization, 'Vikings');
  assert.equal(entries[0].title, 'Research');
  assert.deepEqual(
    entries[0].tokens.map((t) => [t.icon, t.value]),
    [
      ['action', '−1'],
      ['food', '−2'],
      ['research', '+1'],
      ['event', undefined],
      ['wonder', '+1'],
    ],
  );
  assert.ok(entries[0].text.includes('2 left'));
  assert.ok(!entries[0].text.includes('Leif'));
  assert.ok(!entries[0].text.includes('Hidden'));
});

test('journal preserves signs for three-resource piles, separate payments, and city mood changes', () => {
  const [entry] = journal(
    game([
      {
        action: { Playing: { Collect: {} } },
        log: [
          'Leif: Collect: Pay 1 action, Use city D2, Gain 1 food, 2 ore and 3 wood, Pay 1 culture token, City D2 became Neutral',
        ],
      },
    ]),
  );
  assert.equal(entry.title, 'Collect · D2');
  assert.deepEqual(
    entry.tokens.map((t) => [t.icon, t.value]),
    [
      ['action', '−1'],
      ['food', '+1'],
      ['ore', '+2'],
      ['wood', '+3'],
      ['culture_tokens', '−1'],
      ['neutral', undefined],
    ],
  );
  assert.deepEqual(entry.notes, []);
});

test('journal keeps chronology and unknown rule details, distinguishes factions and old names', () => {
  const entries = journal(
    game([
      {
        action: { Playing: { ActionCard: {} } },
        log: [
          'Player1: Trade: Gain 1 food from Aurelia',
          'Aurelia: Trade: Lose 1 food',
          'Leif: Trade: Complete Legacy using an objective card',
          'A sudden storm ends movement.',
        ],
      },
    ]),
  );
  assert.deepEqual(
    entries.map((e) => e.civilization),
    ['Vikings', 'Rome', 'Vikings', undefined],
  );
  assert.ok(entries[0].text.includes('Gain 1 food from Rome'));
  assert.ok(entries[2].text.includes('Complete Legacy using an objective card'));
  assert.equal(entries[3].text, 'A sudden storm ends movement.');
  assert.equal(new Set(entries.map((e) => e.id)).size, entries.length);
});

test('origin metadata resolves renamed players while unrelated effects retain their source', () => {
  const entries = journal(
    game([
      {
        log: ['Old name: Engineering: Draw a wonder card', 'Old name: Storage: Gain 1 food'],
        items: [
          { player: 0, origin: { Advance: 'Engineering' } },
          { player: 0, origin: { Advance: 'Storage' } },
        ],
      },
    ]),
  );
  assert.deepEqual(
    entries.map((e) => [e.civilization, e.title]),
    [
      ['Vikings', 'Engineering'],
      ['Vikings', 'Storage'],
    ],
  );
});

test('objective resource formatting preserves condition text and amounts', () => {
  const text = 'You have at least 3 food, 3 ore, and 3 wood.';
  const parts = journalParts(text);
  assert.equal(parts.map((p) => p.text).join(''), text);
  assert.deepEqual(
    parts.filter((p) => p.resource).map((p) => p.resource),
    ['food', 'ore', 'wood'],
  );
});

test('card payment announcements, including repeated starts, never count as culture spent', () => {
  const announcement: LoggedAction = {
    action: { Playing: { ActionCard: 27 } },
    items: [{ player: 0, HandCard: { to: 'PlayToDiscard' }, origin: { Ability: 'Action Card' } }],
    log: ['Leif: Action Card: Play Assassination/High Ground', 'Leif: Assassination: Pay 1 culture token'],
  };
  const payment: LoggedAction = {
    action: { Response: { Payment: [{ culture_tokens: 1 }] } },
    items: [
      {
        player: 0,
        Resources: { resources: { culture_tokens: 1 }, balance: 'Loss' },
        origin: { CivilCard: 27 },
      },
    ],
    log: ['Leif: Assassination: Pay 1 culture token, Assassinated Aurelia'],
  };
  assert.equal(journal(game([announcement])).flatMap((e) => e.tokens).length, 0);
  const entries = journal(game([announcement, structuredClone(announcement), payment]));
  assert.deepEqual(
    entries.flatMap((e) => e.tokens).map((t) => [t.icon, t.value]),
    [['culture_tokens', '−1']],
  );
  assert.equal(entries.filter((e) => e.title === 'Assassination').length, 1);
  assert.ok(entries.some((e) => e.notes.includes('Assassinated Aurelia')));
});

test('shared happiness fees and substituted custom-action payments use actual resource losses', () => {
  const state = game([
    {
      action: { Playing: { IncreaseHappiness: { action_type: { Custom: 'VotingIncreaseHappiness' } } } },
      items: [{ player: 0, Resources: { resources: { mood_tokens: 5 }, balance: 'Loss' } }],
      log: ['Leif: Voting: Pay 1 mood token, Pay 5 mood tokens, City D2 became Happy, City E4 became Happy'],
    },
    {
      action: { Playing: { Custom: { action: 'TheArtOfWar' } } },
      log: ['Leif: The Art of War: Pay 1 mood token'],
    },
    {
      action: { Response: { Payment: [{ culture_tokens: 1 }] } },
      items: [{ player: 0, Resources: { resources: { culture_tokens: 1 }, balance: 'Loss' } }],
      log: ['Leif: The Art of War: Pay 1 culture token'],
    },
  ]);
  const entries = journal(state);
  assert.deepEqual(
    entries
      .flatMap((e) => e.tokens)
      .filter((t) => t.tone === 'loss')
      .map((t) => [t.icon, t.value]),
    [
      ['mood_tokens', '−5'],
      ['culture_tokens', '−1'],
    ],
  );
  assert.equal(entries.flatMap((e) => e.tokens).filter((t) => t.icon === 'happy').length, 2);
});
