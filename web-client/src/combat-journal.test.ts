import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { journal } from './model.ts';
import { combatJournal } from './combat-journal.ts';
import { activeCombat } from './active-combat.ts';
import type { Game, JournalEntry } from './types.ts';

function fixture(name: string): Game {
  return JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8'),
  );
}
const rounds = (entries: JournalEntry[]) =>
  combatJournal(entries).flatMap((entry) => (entry.combat ? [entry.combat] : []));

test('unfinished combat compares both armies and uses engine hits, including the enemy unit cap', () => {
  const game = fixture('remove_casualties_attacker.outcome');
  const entries = journal(game),
    snapshot = JSON.stringify(entries);
  const [combat] = rounds(entries);
  assert.equal(combat.attacker.civilization, 'Rome');
  assert.deepEqual(
    combat.attacker.units?.map((unit) => [unit.type, unit.count]),
    [
      ['Infantry', 2],
      ['Cavalry', 1],
      ['Elephant', 1],
    ],
  );
  assert.deepEqual(
    combat.attacker.dice?.map((die) => die.value),
    [6, 6, 6, 6],
  );
  assert.equal(combat.attacker.value, 26);
  assert.equal(combat.attacker.hits, 2); // Capped by two defending fighters, not floor(26/5).
  assert.equal(combat.defender.value, 14);
  assert.equal(combat.defender.hits, 2);
  assert.equal(combat.attacker.cancelledHits, undefined, 'The enemy unit cap is not a cancelled hit');
  assert.equal(combat.result, undefined);
  assert.equal(JSON.stringify(entries), snapshot);
});

test('Sun Shield shows the original enemy hit beside the already-cancelled result', () => {
  const game = fixture('remove_casualties_attacker.outcome');
  game.log![0].rounds[0].turns[0].actions = [
    {
      log: [
        'Combat round 1',
        'Player1: Combat: Attacking with 1 infantry',
        'Player2: Combat: Defending with Pakal',
        'Player1: Combat: Roll 5 (infantry, +1 combat value) for combined combat value of 6 and gets 0 hits against defending units',
        'Player2: Combat: Roll 6 (infantry, no bonus) for combined combat value of 6 and gets 1 hits against attacking units, Combat modifiers: Sun Shield cancels 1 hit',
        'Defender wins',
      ],
    },
  ];
  game.log_index = 1;
  const entries = journal(game),
    original = JSON.stringify(entries);
  const [combat] = rounds(entries);
  assert.equal(combat.attacker.hits, 0, 'Keep the authoritative result; do not apply Sun Shield twice');
  assert.deepEqual(combat.attacker.cancelledHits, { before: 1, reasons: ['Sun Shield cancels 1 hit'] });
  assert.equal(combat.defender.hits, 1, 'Maya still deals its hit');
  assert.equal(combat.defender.cancelledHits, undefined);
  assert.deepEqual(combat.defender.modifiers, ['Sun Shield cancels 1 hit']);
  assert.equal(JSON.stringify(entries), original);

  game.log![0].rounds[0].turns[0].actions[0].log = game.log![0].rounds[0].turns[0].actions[0].log!.filter(
    (line) => !line.includes('Defending with'),
  );
  assert.equal(
    rounds(journal(game))[0].attacker.cancelledHits,
    undefined,
    'Unknown unit caps are not guessed',
  );
});

test('rosters, tactics and rolls combine across payment prompts without treating captures as modifiers', () => {
  const entries = journal(fixture('combat_all_modifiers.outcome5'));
  const output = combatJournal(entries),
    [combat] = rounds(entries);
  assert.equal(combat.attacker.tactics, 'Peltasts');
  assert.equal(combat.defender.tactics, 'Encircled');
  assert.deepEqual(combat.attacker.modifiers, [
    'steel weapons added 1 combat value',
    'Peltasts rolls a 6 and ignored a hit',
  ]);
  assert.equal(combat.attacker.value, 22);
  assert.equal(combat.result, 'Rome wins');
  assert.ok(combat.outcomes.some((entry) => entry.notes.includes('Gain Fortress at C1')));
  assert.ok(combat.outcomes.some((entry) => entry.tokens.some((token) => token.label === 'City C1')));
  assert.equal(output.filter((entry) => entry.title === 'Steel Weapons').length, 2);
  assert.ok(output.some((entry) => entry.title === 'Encircled' && entry.notes.length));
});

test('elephant blocks, leader rerolls and unknown loss text survive formatting', () => {
  const [combat] = rounds(journal(fixture('direct_capture_city_metallurgy.outcome')));
  assert.deepEqual(combat.attacker.dice?.[0], {
    value: 2,
    symbol: 'Elephant',
    effect: '-1 hits, no combat value',
  });
  assert.deepEqual(combat.defender.dice?.[0], { value: 1, symbol: 'Leader', effect: 're-roll' });
  assert.deepEqual(combat.defender.units?.[0].type, { Leader: 'Caesar' });
  assert.equal(combat.defender.hits, 0);
  assert.ok(combat.outcomes.some((entry) => entry.notes.includes('Lose Caesar at C1')));
});

test('each round and each separate battle retains its own casualties and result', () => {
  const battle = rounds(journal(fixture('retreat_no.outcome1')));
  assert.deepEqual(
    battle.map((round) => round.round),
    [1, 2],
  );
  assert.deepEqual(
    battle.map((round) => round.attacker.value),
    [1, 7],
  );
  assert.equal(battle[0].result, undefined);
  assert.equal(battle[1].result, 'Greece wins');
  assert.equal(
    battle[1].outcomes.flatMap((entry) => entry.tokens).filter((token) => token.icon === 'unit').length,
    2,
  );
  const pirates = rounds(journal(fixture('recruit_combat.outcome6')));
  assert.equal(pirates.length, 2);
  assert.deepEqual(
    pirates.map((round) => round.round),
    [1, 1],
  );
  assert.deepEqual(
    pirates.map((round) => round.attacker.value),
    [24, 12],
  );
});

test('naval rolls retain inactive die faces and fortress-only defense can roll without units', () => {
  const [naval] = rounds(journal(fixture('ship_combat.outcome')));
  assert.deepEqual(naval.attacker.dice, [
    { value: 6, symbol: 'Infantry' },
    { value: 6, symbol: 'Infantry' },
  ]);
  const [fortress] = rounds(journal(fixture('direct_capture_city_only_fortress.outcome')));
  assert.deepEqual(fortress.defender.units, []);
  assert.equal(fortress.defender.dice?.length, 1);
  assert.ok(fortress.defender.modifiers.includes('fortress added one extra die'));
});

test('identical numbers preserve different faces and distinguish used abilities from inactive faces', () => {
  const game = fixture('remove_casualties_attacker.outcome');
  game.log![0].rounds[0].turns[0].actions = [
    {
      log: [
        'Player1: Combat: Roll 4 (cavalry, +2 combat value), 4 (elephant, -1 hits, no combat value), 4 (cavalry, no bonus) for combined combat value of 10 and gets 1 hits against defending units',
      ],
    },
  ];
  game.log_index = 1;
  const [combat] = rounds(journal(game));
  assert.deepEqual(combat.attacker.dice, [
    { value: 4, symbol: 'Cavalry', effect: '+2 combat value' },
    { value: 4, symbol: 'Elephant', effect: '-1 hits, no combat value' },
    { value: 4, symbol: 'Cavalry' },
  ]);
});

test('older numeric-only logs remain readable without guessing a face from the number', () => {
  const entry: JournalEntry = {
    id: '0-0-0-0-0',
    age: 1,
    round: 1,
    kind: 'combat',
    title: 'Combat',
    player: 0,
    civilization: 'Rome',
    tokens: [],
    notes: ['Roll 4 → combat value 4 → 0 hits against defending units'],
    text: '',
  };
  const [combat] = rounds([entry]);
  assert.deepEqual(combat.attacker.dice, [{ value: 4 }]);
});

test('undo truncation is reflected without keeping later results in the projection', () => {
  const game = fixture('combat_all_modifiers.outcome5');
  game.log_index = 1;
  const [combat] = rounds(journal(game));
  assert.equal(combat.attacker.units?.length, 4);
  assert.equal(combat.attacker.value, undefined);
  assert.equal(combat.attacker.tactics, undefined);
  assert.equal(combat.result, undefined);
});

test('unfamiliar rolls stay visible, including when no round marker is available', () => {
  const entry: JournalEntry = {
    id: '0-0-0-0-0',
    age: 1,
    round: 1,
    kind: 'combat',
    title: 'Combat',
    player: 0,
    civilization: 'Rome',
    tokens: [],
    notes: ['Roll unusual dice → combat value 4 → 0 hits against defending units'],
    text: '',
  };
  const output = combatJournal([entry]);
  assert.deepEqual(output, [entry]);
});

test('active combat is found below nested decisions and disappears after combat ends', () => {
  const game = fixture('remove_casualties_attacker.outcome');
  const original = JSON.stringify(game);
  const combat = activeCombat(game)!;
  assert.deepEqual(combat.attacker.position, 'C2');
  assert.deepEqual(combat.defender.position, 'C1');
  assert.equal(JSON.stringify(game), original);
  assert.equal(activeCombat({ events: [...game.events!, { event_type: { Ability: {} } }] }), combat);
  assert.equal(activeCombat({ events: [...game.events!, { event_type: { CombatEnd: combat } }] }), null);
  assert.equal(activeCombat({ events: [] }), null);
  assert.equal(
    activeCombat({ events: [{ event_type: 'TurnStart' }, { event_type: 'ChooseActionCard' }] }),
    null,
  );
  assert.equal(activeCombat({}), null);
  assert.equal(activeCombat({ events: [{ event_type: { CombatStart: { stats: combat } } }] }), combat);
  assert.equal(
    activeCombat({ events: [{ event_type: { CombatRoundStart: { combat: { stats: combat } } } }] }),
    combat,
  );
});
