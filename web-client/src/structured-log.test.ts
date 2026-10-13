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


test('declining government change omits the empty payment from the journal', () => {
  const game = fixture('status_phase/keep_government.outcome1');
  const before = JSON.stringify(game);
  const entry = journal(game).find(e => e.title === 'Change Government');
  assert.ok(entry);
  assert.match(JSON.stringify(entry), /Keep current government/);
  assert.doesNotMatch(JSON.stringify(entry), /Pay ,|Pay /);
  assert.equal(JSON.stringify(game), before);
});

test('played action cards and their research results link to the exact public card', () => {
  const game = fixture('action_cards/advance.outcome2');
  const before = JSON.stringify(game);
  const card = { id: 2, name: 'Advance', description: 'Gain an advance.', free: true, tactics: null };
  const entries = journal(game, { players: [], cardCatalog: [
    { ...card, id: 99 }, card,
  ], logOriginNames: { '{"CivilCard":2}': 'Advance' } });
  const played = entries.filter(entry => entry.title === 'Action card · Advance');
  assert.equal(played.length, 3);
  assert.ok(played.every(entry => entry.kind === 'card' && entry.card?.id === 2));
  assert.ok(played.some(entry => entry.tokens.some(token => token.value === '−1')));
  assert.ok(played.some(entry => /Storage/.test(entry.text)));
  assert.ok(!entries.some(entry => entry.title === 'Action Card'));
  assert.equal(JSON.stringify(game), before);
});

test('redacted card removal resolves the public played card while hidden discards stay unnamed', () => {
  const game = fixture('action_cards/advance.outcome2');
  const actions = game.log![0].rounds[0].turns[0].actions!;
  const played = actions[0];
  played.items![0].HandCard!.card = {ActionCard:0};
  const card = {id:2,name:'Advance',description:'Gain an advance.',free:true,tactics:null};
  const context = {players:[],cardCatalog:[card],logOriginNames:{'{"CivilCard":2}':'Advance'}};
  let entries = journal(game,context);
  assert.equal(entries[0].card?.id,2);
  assert.equal(entries[0].title,'Action card · Advance');
  assert.ok(!entries.some(entry=>/Card 0/.test(entry.text)));
  played.action = {Response:{SelectHandCards:[]}};
  entries = journal(game,context);
  assert.equal(entries[0].card,undefined);
  assert.equal(entries[0].title,'Action Card');
  assert.ok(!entries.some(entry=>/Card 0/.test(entry.text)));
});
