import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { journal } from './journal.ts';
import { combatJournal } from './combat-journal.ts';
import type { Game } from './types.ts';
const fixture = (name: string): Game =>
  JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));

test('structured collection records keep gains, waste and included bonuses readable without modifying engine data', () => {
  const game = fixture('base/collect.outcome');
  const before = JSON.stringify(game);
  const entries = journal(game);
  const collection = entries.find((entry) => entry.collection);
  assert.ok(collection);
  assert.equal(collection.player, 0);
  assert.equal(collection.collection?.tiles.length, 2);
  assert.ok(collection.tokens.some((token) => token.icon === 'ideas' && token.value === '+1'));
  assert.ok(
    !entries.some((entry) => entry.title === 'Public Education'),
    'Included bonus is not counted twice',
  );
  assert.ok(
    entries.some(
      (entry) =>
        entry.title === 'Waste' && entry.tokens.some((token) => token.icon === 'ore' && token.value === '−1'),
    ),
  );
  assert.equal(JSON.stringify(game), before);
});

test('structured combat records retain participants, dice and rule modifiers', () => {
  const game = fixture('combat/direct_capture_city_metallurgy.outcome');
  const combat = combatJournal(journal(game)).find((entry) => entry.combat)?.combat;
  assert.ok(combat);
  assert.equal(combat.attacker.player, 0);
  assert.equal(combat.defender.player, 1);
  assert.ok(combat.attacker.dice?.length);
  assert.ok(
    combat.attacker.modifiers.includes('steel weapons added 2 combat value (Metallurgy: no ore cost)'),
  );
});
