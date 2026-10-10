import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { printedCardName, printedCardPart, printedCardText } from './card-names.ts';
import { namedTextParts } from './research-links.ts';
import { cardReferences } from './card-reference.ts';
import { createTranslator } from './localization/runtime.js';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('legacy objective hands and completed journal links keep their identifiers with printed titles', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'printed-card-names', {}));
  game.players[0].objective_cards = [34, 28, 2, 22];
  game.players[0].completed_objectives = [{ card: 34, name: 'Traders' }];
  const state = JSON.stringify(game);
  const view: View = JSON.parse(engine.webView(engine.stripSecret(state, 0), 0));
  const titles = view.objectiveCards.map((c) => [
    c.id,
    ...c.objectives.map((o) => printedCardName(o.name, 'objective')),
  ]);
  assert.deepEqual(titles, [
    [34, 'Mercantile', 'Great Commander'],
    [28, 'Trader', 'Defiance'],
    [2, 'Academic', 'Conqueror'],
    [22, 'Citadel', 'Warlord'],
  ]);
  const target = { name: view.players[0].completedObjectives![0].name, player: 0 };
  const parts = namedTextParts('Complete objective Traders', [target]).map(({ text, reference }) =>
    printedCardPart({ text, objective: reference }),
  );
  assert.equal(parts.map((p) => p.text).join(''), 'Complete objective Mercantile');
  assert.equal(parts[1].objective, target);
  assert.equal(parts[1].objective!.name, 'Traders');
  assert.equal(JSON.stringify(game), state);
});

test('printed action and tactics references retain the original card and playable ID', async () => {
  const state = await engine.init(2, [], {}, 'printed-action-names', {});
  const view: View = JSON.parse(engine.webView(engine.stripSecret(state, 0), 0));
  const refs = cardReferences(view);
  const parts = namedTextParts('Great Ideas or High Ground; Technology Trade; Encircled', refs).map(
    ({ text, reference }) => printedCardPart({ text, card: reference }),
  );
  assert.equal(parts.map((p) => p.text).join(''), 'Good Ideas or High Ground; Tech Exchange; Routing');
  assert.equal(parts.find((p) => p.text === 'Good Ideas')!.card!.card.id, 12);
  assert.equal(parts.find((p) => p.text === 'Tech Exchange')!.card!.card.id, 40);
  assert.equal(parts.find((p) => p.text === 'Routing')!.card!.card.tactics!.name, 'Encircled');
});

test('sentence labels preserve resources and already-correct titles without cascading replacements', () => {
  assert.equal(
    printedCardText('Revolution: Anarchy; Tactical Surprise; The Siege; Teach Us Now!'),
    'Revolution: Anarchy; Tactical Surprise; The Siege; Teach Us Now!',
  );
  assert.equal(printedCardName('Migration'), 'Migration');
  assert.equal(printedCardName('Migration', 'objective'), 'Great Migration');
  assert.equal(
    printedCardText('Hero General: Great Ideas grants 2 Ideas.'),
    'Heroic General: Good Ideas grants 2 Ideas.',
  );
  assert.equal(
    printedCardText('Heroic General and Good Ideas. NotTraders stays plain.'),
    'Heroic General and Good Ideas. NotTraders stays plain.',
  );
  assert.equal(printedCardPart({ text: 'Ideas', resource: 'ideas' }).text, 'Ideas');
  assert.equal(printedCardPart({ text: 'Ideas', card: { name: 'Ideas' } }).text, 'Leap of Knowledge');
});

test('every scanned card title has a matching display alias, while legacy translations remain available', () => {
  const audit = JSON.parse(readFileSync(new URL('../../docs/card-text-audit.json', import.meta.url), 'utf8'));
  for (const c of audit.objective_cards)
    for (const o of c.objectives) assert.equal(printedCardName(o.name, 'objective'), o.printed_name);
  for (const c of audit.action_cards) {
    assert.equal(printedCardName(c.name), c.printed_name);
    assert.equal(printedCardName(c.tactics), c.printed_tactics);
  }
  for (const c of audit.event_cards) assert.equal(printedCardName(c.name), c.printed_name);
  const fr = JSON.parse(readFileSync(new URL('./localization/fr.json', import.meta.url), 'utf8'));
  const translate = createTranslator({ fr }, 'fr');
  for (const name of ['Traders', 'Great Ideas', 'Technology Trade', 'Epidemics'])
    assert.equal(translate.translate(printedCardName(name)), translate.translate(name));
});
