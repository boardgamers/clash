import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTranslator } from './localization/runtime.js';
import { createTranslator as createLegacyTranslator } from '../../client/js/src/localization/runtime.js';

const catalogs = {
  fr: {
    '{p0} at {p1}': '{p0} à {p1}',
    '{p0} with {p1}': '{p0} avec {p1}',
    '{p0}. {p1}': '{p0}. {p1}',
    '{p0} at {p1} traded with city {p2}': '{p0} à {p1} a commercé avec la ville {p2}',
    Infantry: 'Infanterie',
    'You can recruit units at this city after completing the required action.':
      'Vous pouvez recruter des unités dans cette ville après avoir terminé l’action requise.',
  },
};

for (const [client, create] of [
  ['web', createTranslator],
  ['legacy', createLegacyTranslator],
] as const) {
  test(`${client}: broad connectors leave unknown prose intact`, () => {
    const translator = create(catalogs, 'fr');
    for (const text of [
      'You can recruit additional units at this city after completing the required action.',
      'Infantry with a newly introduced ability that is not yet in the translation catalog',
      'An entirely new rule appears here. Infantry',
      'Infantry with friends!',
    ]) {
      assert.equal(translator.translate(text), text);
      assert.equal(translator.translate(text), text, 'cached fallback stays intact');
    }
  });

  test(`${client}: short captions, names, and complete templates still translate`, () => {
    const translator = create(catalogs, 'fr');
    assert.equal(translator.translate('Infantry at A1'), 'Infanterie à A1');
    assert.equal(translator.translate('Infantry with Alice'), 'Infanterie avec Alice');
    const name = 'A player whose very long name includes punctuation!';
    translator.setNames([name]);
    assert.equal(translator.translate(`${name} at A1`), `${name} à A1`);
    assert.equal(translator.translate(name), name);
    assert.equal(
      translator.translate(`${name} at A1 traded with city B2`),
      `${name} à A1 a commercé avec la ville B2`,
    );
    assert.equal(
      translator.translate('You can recruit units at this city after completing the required action.'),
      'Vous pouvez recruter des unités dans cette ville après avoir terminé l’action requise.',
    );
  });

  test(`${client}: opt-in authored-key coverage ignores broad matching and gameplay text`, () => {
    const translator = create(catalogs, 'fr');
    translator.setNames(['Private player name']);
    translator.translate('Private player name with Infantry');
    translator.translate('Unrecognized runtime content');
    assert.deepEqual(
      translator.getMissingKeys(['Infantry', '{p0} at {p1}', 'New authored caption', 'New authored caption']),
      ['New authored caption'],
    );
    assert.deepEqual(translator.getMissingKeys([]), []);
    translator.setLocale('de');
    assert.deepEqual(translator.getMissingKeys(['Infantry']), ['Infantry']);
  });
}
