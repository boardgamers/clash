import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resourceExplanation } from './resource-log.ts';
import { readableHistory } from './structured-log.ts';
import { journal } from './journal.ts';
import type { Game, LoggedAction, Pile } from './types.ts';

type Item = NonNullable<LoggedAction['items']>[number];
const origin = { Incident: 49 };
const record = (resources: Pile, balance: 'Pay' | 'Loss' | 'Gain' = 'Pay'): Item => ({ player: 0, origin, Resources: { resources, balance } });
const prose = (Text: string): Item => ({ player: 0, origin, Text });
const history = (actions: LoggedAction[]): Game => ({
  state: 'Playing', map: { tiles: [] }, current_player_index: 0,
  age: 2, round: 1, actions_left: 1, log_index: actions.length,
  players: [{ id: 0, name: 'Player1', civilization: 'Rome' }],
  log: [{ age: 2, rounds: [{ round: 1, turns: [{ turn_type: { Player: 0 }, actions }] }] }],
});
const lines = (actions: LoggedAction[]) => readableHistory(history(actions)).log![0].rounds[0].turns[0].actions!.flatMap(a => a.log!);

for (const [resource, label] of Object.entries({ food: 'food', wood: 'wood', ore: 'ore', gold: 'gold', ideas: 'idea', mood_tokens: 'mood token', culture_tokens: 'culture token', captives: 'captive' })) {
  for (const [balance, verb] of [['Pay', 'Pay'], ['Pay', 'Paid'], ['Pay', 'Lose'], ['Pay', 'Lost'], ['Loss', 'Lose'], ['Loss', 'Lost'], ['Gain', 'Gain'], ['Gain', 'Gained']] as const) {
    test(`${balance}/${verb}: ${resource} is rendered once, legacy and repeated transactions survive`, () => {
      const structured = record({ [resource]: 1 }, balance), text = prose(`${verb} 1 ${label}`);
      const game = history([{ items: [structured, text] }]), before = JSON.stringify(game);
      const tokens = journal(game).flatMap(e => e.event ? e.event.outcomes.flatMap(o => o.tokens) : e.tokens);
      assert.equal(tokens.filter(t => t.icon === resource).length, 1);
      assert.equal(JSON.stringify(game), before);
      assert.equal(lines([{ items: [structured, text] }]).length, 1);
      assert.equal(lines([{ items: [structured, structured] }]).length, 2);
      assert.equal(lines([{ items: [structured, text, structured, text] }]).length, 2);
      assert.equal(lines([{ items: [text] }]).length, 1);
      const tokenCount = (actions: LoggedAction[]) => journal(history(actions)).flatMap(e => e.event ? e.event.outcomes.flatMap(o => o.tokens) : e.tokens).filter(t => t.icon === resource).length;
      assert.equal(tokenCount([{ items: [structured, structured] }]), 2);
      assert.equal(tokenCount([{ items: [text] }]), 1);
      assert.equal(tokenCount([{ log: [`Player1: Pandemics: ${text.Text}`] }]), 1);
      assert.equal(lines([{ log: [`Player1: Pandemics: ${text.Text}`] }]).length, 1);
      assert.equal(lines([{ items: [structured] }, { items: [text] }]).length, 2);
    });
  }
}

test('reported Pandemics and Viking Raids payment-loss echoes use exact scoped piles', () => {
  for (const source of [origin, { Ability: 'Viking Raids' }]) {
    assert.equal(resourceExplanation({ ...prose('Lose 1 gold'), origin: source }, { ...record({ gold: 1 }), origin: source }), undefined);
  }
  assert.equal(resourceExplanation(prose('Lose 1 gold'), record({ gold: 1 })), undefined);
  assert.equal(resourceExplanation({ ...prose('Lose 1 gold'), origin: { Incident: 50 } }, { ...record({ gold: 1 }), origin: { Incident: 50 } }), 'Lose 1 gold');
  assert.equal(resourceExplanation(prose('Pay 1 gold'), record({ gold: 1 }, 'Loss')), 'Pay 1 gold');
});

test('mixed piles are compared in full with aliases, zero fields, order and retained explanations', () => {
  const actual = record({ ideas: 1, mood_tokens: 2, food: 0, captives: 1 });
  assert.equal(resourceExplanation(prose('Paid 1 captive, 2 mood tokens and 1 idea with a discount'), actual), 'With a discount');
  assert.equal(resourceExplanation(prose('Pay 1 idea + 1 mood token + 1 mood token and 1 captive'), actual), undefined);
  for (const Text of ['Pay 1 idea', 'Pay 1 idea and 2 mood tokens', 'Pay 2 ideas and 2 mood tokens and 1 captive', 'Pay 1 resource', 'Pay 1 idea if possible', 'Added 1 idea', 'Gain 1 victory point']) {
    assert.equal(resourceExplanation(prose(Text), actual), Text);
  }
});

test('correlation does not cross origin, player, item, action or transaction semantics', () => {
  const actual = record({ gold: 1 }), text = prose('Pay 1 gold');
  for (const changed of [{ ...text, player: 1 }, { ...text, origin: { Incident: 50 } }, { ...text, origin: undefined }]) {
    assert.equal(resourceExplanation(changed, actual), text.Text);
  }
  assert.equal(resourceExplanation({ ...text, origin: {} }, { ...actual, origin: {} }), text.Text);
  assert.equal(resourceExplanation({ ...text, origin: undefined }, { ...actual, origin: undefined }), text.Text);
  assert.equal(resourceExplanation(text, record({ gold: 1 }, 'Gain')), text.Text);
  assert.equal(lines([{ items: [text, actual] }]).length, 2);
  assert.equal(lines([{ items: [actual, prose('A separate effect'), text] }]).length, 3);
  assert.equal(lines([{ items: [actual, record({ wood: 1 }), text] }]).length, 3);
  assert.equal(lines([{ items: [actual, text, text] }]).length, 2);
  assert.equal(lines([{ log: ['Player1: Event: Pay 1 gold', 'Player1: Event: Pay 1 gold'] }]).length, 2);
  assert.equal(resourceExplanation(prose('Added 1 gold'), record({ gold: 1 }, 'Gain')), 'Added 1 gold');
});

test('real checked-in Pandemics and Raids logs normalize without changing state or structured records', () => {
  for (const name of ['incidents/pandemics/pandemics.outcome3', 'civilizations/vikings/raids.outcome2']) {
    const game: Game = JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
    const before = JSON.stringify(game);
    let echoes = 0;
    for (const age of game.log!) for (const round of age.rounds) for (const turn of round.turns) for (const action of turn.actions ?? []) {
      for (const [i, item] of (action.items ?? []).entries()) if (item.Text && resourceExplanation(item, action.items?.[i - 1]) !== item.Text) echoes++;
    }
    assert.ok(echoes > 0, name);
    readableHistory(game);
    assert.equal(JSON.stringify(game), before);
  }
});


test('descriptive objectives, reward-route prose, collection modifiers and waste are retained', () => {
  const objective = { Objective: 'Stockpile' };
  const objectiveText = { ...prose('Pay 1 gold for Stockpile'), origin: objective };
  assert.equal(resourceExplanation(objectiveText, { ...record({ gold: 1 }), origin: objective }), objectiveText.Text);
  // Reward callbacks run BEFORE the gain; the only custom callback emits trade
  // route descriptions, not duplicate resource amounts. Do not infer backwards.
  assert.equal(lines([{ items: [prose('Settler at A1 traded with city B2'), record({ gold: 1 }, 'Gain')] }]).length, 2);
  assert.equal(lines([{ items: [prose('Gain 1 gold'), record({ gold: 1 }, 'Gain')] }]).length, 2);
  assert.equal(lines([{ items: [record({ gold: 1 }, 'Gain'), { ...record({ gold: 1 }, 'Loss'), origin: { Ability: 'Waste' } }] }]).length, 2);
  assert.equal(resourceExplanation({ ...prose('Gain 1 idea'), origin: { Advance: 'PublicEducation' } }, record({ ideas: 1 }, 'Gain')), 'Gain 1 idea');
});
