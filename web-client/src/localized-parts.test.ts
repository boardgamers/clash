import { test } from 'node:test';
import assert from 'node:assert/strict';
import { localizedParts } from './localized-parts.ts';
import { readFileSync } from 'node:fs';
import { createTranslator, languages } from './localization/runtime.js';
import { journalParts } from './model.ts';

test('Great Person acquisition translates both sentences and preserves culture costs', () => {
  const source = 'The player who triggered the event may take the card for 1 culture token. If they pass, other players may take it in turn order for 2 culture tokens.';
  for (const locale of Object.keys(languages)) {
    const catalog = JSON.parse(readFileSync(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    assert.ok(Object.hasOwn(catalog, source), `${locale}: complete acquisition rule`);
    if (locale !== 'en') assert.notEqual(catalog[source], source);
    const parts = localizedParts(source, journalParts(source), translator.translate);
    assert.equal(parts.map(p => p.text).join(''), catalog[source]);
    const resources = parts.filter(p => 'resource' in p);
    assert.equal(resources.length, 2, `${locale}: both resource icons survive translation`);
    assert.ok(resources.every(p => 'resource' in p && p.resource === 'culture_tokens'));
    if (locale === 'fr') {
      assert.equal(parts.map(p => p.text).join(''),
        "Le joueur qui a déclenché l'événement peut prendre la carte pour 1 jeton de culture. S'il renonce, les autres joueurs peuvent la prendre, dans l'ordre du tour, pour 2 jetons de culture.");
      assert.deepEqual(resources.map(p => p.text), ['1 jeton de culture', '2 jetons de culture']);
    }
  }
});

test('translates the whole sentence and keeps reordered references and IDs', () => {
  const catalog: Record<string, string> = {
    'Pay 2 food to research Math.': 'Math erforschen kostet 2 Nahrung.',
    Math: 'Math',
    '2 food': '2 Nahrung',
  };
  const result = localizedParts(
    'Pay 2 food to research Math.',
    [
      { text: 'Pay ' },
      { text: '2 food', resource: 'food' },
      { text: ' to research ' },
      { text: 'Math', research: { id: 'Math', name: 'Math' } },
      { text: '.' },
    ],
    (text) => catalog[text] ?? text,
  );
  assert.equal(result.map((part) => part.text).join(''), 'Math erforschen kostet 2 Nahrung.');
  assert.deepEqual(result[0], { text: 'Math', research: { id: 'Math', name: 'Math' } });
  assert.deepEqual(result[2], { text: '2 Nahrung', resource: 'food' });
});

test('translates segments when no complete source sentence exists', () => {
  assert.deepEqual(
    localizedParts(
      'Unknown sentence food',
      [{ text: 'Unknown sentence ' }, { text: 'food', resource: 'food' }],
      (text) => (text === 'food' ? 'Nahrung' : text),
    ),
    [{ text: 'Unknown sentence ' }, { text: 'Nahrung', resource: 'food' }],
  );
});

test('preserves complete prose when grammar inflects a reference label', () => {
  const result = localizedParts(
    'Pay food.',
    [{ text: 'Pay ' }, { text: 'food', resource: 'food' }, { text: '.' }],
    (text) => (text === 'Pay food.' ? 'Payer de la nourriture.' : text === 'food' ? 'Nourriture' : text),
  );
  assert.equal(result.map((part) => part.text).join(''), 'Payer de la nourriture.');
});

test('does not attach a short advance reference inside a longer translated word', () => {
  const result=localizedParts('A lawgiver knows Law.',[{text:'A lawgiver knows '},{text:'Law',research:{id:'Law'}},{text:'.'}],(text)=>text==='A lawgiver knows Law.'?'Ein Rechtgeber kennt Recht.':text==='Law'?'Recht':text);
  assert.deepEqual(result,[{text:'Ein Rechtgeber kennt '},{text:'Recht',research:{id:'Law'}},{text:'.'}]);
});
