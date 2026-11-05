import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { ageCount, ageLabel, lengthRule, lengthName, variableEndThreshold } from './game-length.ts';
import type { Game } from './types.ts';

const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');

test('published creation choices match the approved modes and preserve the default', async () => {
  const option = JSON.parse(readFileSync(new URL('../bgs-length-option.json', import.meta.url), 'utf8'));
  const proposal = JSON.parse(
    readFileSync(new URL('../../docs/monumental-length-option.proposal.json', import.meta.url), 'utf8'),
  );
  assert.deepEqual(option, proposal);
  assert.equal(option.default, 'Standard');
  assert.deepEqual(option.items, [
    { name: 'Shorter', label: '4 ages — 2 settlers, 2 free advances per status phase' },
    { name: 'Standard', label: '6 ages' },
    { name: 'RollToEnd', label: '6–7 ages — roll to end' },
    { name: 'WonderfulEnding', label: '6–7 ages — wonderful ending' },
    { name: 'Epic', label: '8 ages' },
  ]);
  for (const item of option.items) {
    const game = JSON.parse(await engine.init(2, [], { length: item.name }, 'creation-options', {}));
    assert.equal(game.options?.length ?? 'Standard', item.name);
  }
  const game = JSON.parse(await engine.init(2, [], {}, 'creation-default', {}));
  assert.equal(game.options?.length ?? 'Standard', option.default);
});

test('saved length controls the track without changing standard or Epic', () => {
  assert.equal(ageCount(null), 6);
  for (const [length, count, label] of [
    ['Standard', 6, 'VI'],
    ['Epic', 8, 'VIII'],
    ['Shorter', 4, 'IV'],
    ['RollToEnd', 7, 'VII'],
    ['WonderfulEnding', 7, 'VII'],
  ] as const) {
    assert.equal(ageCount({ options: { length } } as Game), count);
    assert.equal(ageLabel(count), label);
  }
  assert.equal(variableEndThreshold(6, 1), null);
  assert.deepEqual(
    [
      [6, 2],
      [6, 3],
      [7, 1],
      [7, 2],
      [7, 3],
    ].map(([a, r]) => variableEndThreshold(a, r)),
    [6, 5, 4, 3, 2],
  );
  assert.equal(variableEndThreshold(7, 4), null);
  assert.equal(variableEndThreshold(8, 1), null);
});

test('all active rule captions are translated in every locale', () => {
  const locales = [
    'en',
    'de',
    'fr',
    'pl',
    'ro',
    'el',
    'hi',
    'ru',
    'da',
    'pt-BR',
    'ko',
    'zh-TW',
    'zh-CN',
    'vi',
    'it',
    'nl',
    'fa',
  ];
  for (const locale of locales) {
    const catalog = JSON.parse(
      readFileSync(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'),
    );
    for (const length of ['Standard', 'Epic', 'Shorter', 'RollToEnd', 'WonderfulEnding'] as const) {
      const rule = lengthRule({ options: { length } } as Game);
      assert.ok(catalog[rule], `${locale}: ${length}`);
      if (locale !== 'en') assert.notEqual(catalog[rule], rule);
    }
  }
});

test('WASM accepts each exclusive mode with independent modules and preserves defaults', async () => {
  for (const length of ['Standard', 'Epic', 'Shorter', 'RollToEnd', 'WonderfulEnding']) {
    const text = await engine.init(2, [], { length, variant: 'Builder' }, 'length-web-test', {});
    const game = JSON.parse(text);
    assert.equal(game.options?.length ?? 'Standard', length);
    assert.equal(game.options.variant, 'Builder');
    for (const player of game.players.slice(0, 2)) {
      assert.equal(
        player.units.filter((unit: any) => unit.unit_type === 'Settler').length,
        length === 'Shorter' ? 2 : 1,
      );
    }
    assert.doesNotThrow(() => engine.webView(engine.stripSecret(text, 0), 0));
  }
});

test('visible mode names retain the compatible Standard enum', () => {
  assert.equal(lengthName(null), 'Normal');
  assert.equal(lengthName({ options: { length: 'Standard' } } as Game), 'Normal');
  for (const [length, label] of [
    ['Shorter', 'Shorter'],
    ['RollToEnd', 'Roll to End'],
    ['WonderfulEnding', 'Wonderful Ending'],
    ['Epic', 'Epic'],
  ] as const) {
    assert.equal(lengthName({ options: { length } } as Game), label);
  }
});

test('age-track captions exist in all 17 catalogs', () => {
  const locales = [
    'en',
    'de',
    'fr',
    'pl',
    'ro',
    'el',
    'hi',
    'ru',
    'da',
    'pt-BR',
    'ko',
    'zh-TW',
    'zh-CN',
    'vi',
    'it',
    'nl',
    'fa',
  ];
  for (const locale of locales) {
    const catalog = JSON.parse(
      readFileSync(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'),
    );
    for (const key of [
      'Normal',
      'Wonderful Ending',
      'Normal: 6 ages.',
      'Epic: 8 ages.',
      'Age',
      'Round',
      'End on',
      'Wonder to end',
      'Ends this round',
    ]) {
      assert.ok(catalog[key], `${locale}: ${key}`);
    }
  }
});

test('wonder rule starts at a human-readable age and round', () => {
  assert.equal(
    lengthRule({ options: { length: 'WonderfulEnding' } } as Game),
    'Starting from age 6, round 2, building a wonder ends the game after that round. Then complete objectives.',
  );
});
