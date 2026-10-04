import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { journal } from './journal.ts';
import type { Game, LoggedAction } from './types.ts';

const fixture = (name: string): Game =>
  JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/incidents/${name}.json`, import.meta.url), 'utf8'),
  );
const outcomes = (game: Game) => journal(game).flatMap((entry) => entry.event?.outcomes ?? []);
function history(actions: LoggedAction[]): Game {
  const game = fixture('barbarians_move.outcome1');
  game.log = [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 0 }, actions }] }] }];
  game.log_index = actions.length;
  return game;
}

test('recorded barbarian reinforcements explain the movement event and triggering civilization', () => {
  const game = fixture('barbarians_move.outcome1');
  const before = JSON.stringify(game);
  const gain = outcomes(game).find((entry) => entry.civilization === 'Barbarians')!;
  assert.equal(gain.tokens[0].label, 'infantry at B3');
  assert.match(gain.notes[0], /Reinforced after barbarian movement.*2 land spaces of Rome’s cities/);
  assert.ok(
    journal(game)
      .find((entry) => entry.event)!
      .text.includes(gain.notes[0]),
  );
  assert.equal(JSON.stringify(game), before, 'Presentation leaves recorded history untouched');
  game.players[0].cities = [];
  assert.deepEqual(
    outcomes(game).find((entry) => entry.civilization === 'Barbarians')!.notes,
    gain.notes,
    'Later city losses must not change the historical explanation',
  );
});

test('spawn explanations distinguish the new city army from its extra reinforcement', () => {
  const gains = outcomes(fixture('barbarians_spawn.outcome1')).filter(
    (entry) => entry.civilization === 'Barbarians',
  );
  assert.equal(gains.length, 2);
  assert.match(gains[0].notes[0], /New barbarian city.*1 infantry/);
  assert.equal(gains[1].tokens[0].label, 'elephant at A3');
  assert.match(gains[1].notes[0], /Extra reinforcement.*Barbarians spawn/);
});

for (const name of ['barbarians_attack.outcome', 'barbarians_attack.outcome1'])
  test(`post-battle reinforcement is explained without calling the captured city a new spawn (${name})`, () => {
    const entries = journal(fixture(name));
    const combat = entries.filter((entry) => entry.title === 'Combat');
    assert.ok(
      combat.some((entry) => entry.tokens.some((token) => token.icon === 'city' && token.tone === 'gain')),
    );
    assert.ok(
      combat.every((entry) => entry.notes.every((note) => !/New barbarian city|Reinforced after/.test(note))),
    );
    const gain = entries
      .flatMap((entry) => entry.event?.outcomes ?? [])
      .find((entry) => entry.civilization === 'Barbarians')!;
    assert.match(gain.notes[0], /Reinforced after barbarian movement/);
  });

test('a movement fallback creates a city without claiming post-movement reinforcement', () => {
  const game = history([
    {
      log: [
        'A new game event has been triggered: Population Boom',
        'Player1: Population Boom: Base effect: Barbarians move',
        'Player1: Population Boom: Barbarians cannot move - will try to spawn a new city instead',
        'Barbarians: Population Boom: Gain city A3',
        'Barbarians: Population Boom: Gain 1 infantry at A3',
      ],
    },
  ]);
  const gain = outcomes(game).find((entry) => entry.civilization === 'Barbarians')!;
  assert.match(gain.notes[0], /No army could move.*created a barbarian city with 1 infantry/);
  assert.doesNotMatch(gain.text, /Reinforced after barbarian movement/);
});

test('unrecorded, later card and undone gains never inherit a previous event explanation', () => {
  const game = history([
    {
      log: [
        'A new game event has been triggered: Population Boom',
        'Player1: Population Boom: Base effect: Barbarians move',
      ],
    },
    { log: ['Barbarians: Population Boom: Gain 1 infantry at B3'] },
    { action: { Playing: { ActionCard: {} } }, log: ['Barbarians: Mercenaries: Gain 1 infantry at A3'] },
    {
      log: [
        'A new game event has been triggered: Population Boom',
        'Barbarians: Population Boom: Gain 1 infantry at A3',
      ],
    },
  ]);
  const entries = journal(game);
  assert.match(entries[0].text, /Reinforced after/);
  assert.doesNotMatch(entries.find((entry) => entry.title === 'Mercenaries')!.text, /Reinforced after/);
  assert.doesNotMatch(entries.filter((entry) => entry.event)[1].text, /Reinforced after/);
  game.log_index = 1;
  assert.equal(outcomes(game).filter((entry) => entry.civilization === 'Barbarians').length, 0);
  assert.doesNotMatch(journal(game)[0].text, /Reinforced after/);
});

test('a single log entry can explain both the initial infantry and an extra spawn unit', () => {
  const game = history([
    {
      log: [
        'A new game event has been triggered: A good year',
        'Player1: A good year: Base effect: Barbarians spawn',
        'Barbarians: A good year: Gain city A3',
        'Barbarians: A good year: Gain 2 infantry at A3',
      ],
    },
  ]);
  const gain = outcomes(game).find((entry) => entry.civilization === 'Barbarians')!;
  assert.match(gain.notes[0], /starts with 1 infantry.*also adds an extra unit/);
});
