import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { journal } from './journal.ts';
import { journalParts } from './model.ts';
import { localizedParts } from './localized-parts.ts';
import { createTranslator } from './localization/runtime.js';
import type { Game } from './types.ts';

const fixture = (name: string): Game =>
  JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));

test('journal preserves permanent loss notes, their owner, and forced raze reason without changing history', () => {
  for (const [name, expected] of [
    [
      'incidents/earthquake/volcano.outcome1',
      'Temple at C2 permanently destroyed; total piece limit reduced to 4',
    ],
    [
      'incidents/earthquake/earthquake.outcome3',
      'City center at A3 permanently destroyed; city limit reduced to 5',
    ],
    ['combat/capture_and_raze_city.outcome', 'Razed C1 because no city centers were available'],
  ]) {
    const game = fixture(name);
    const before = JSON.stringify(game);
    const entries = journal(game);
    assert.ok(JSON.stringify(entries).includes(expected), name);
    assert.equal(JSON.stringify(game), before);
    if (name.includes('volcano')) {
      const outcome = entries.flatMap((e) => e.event?.outcomes ?? []).find((e) => e.notes.includes(expected));
      assert.equal(outcome?.player, 1, 'foreign-owned Temple belongs to Greece');
      assert.equal(outcome?.civilization, 'Greece');
    }
  }
});

test('piece capacity and forced raze notes translate as complete messages in all locales', () => {
  const samples: [string, string[]][] = [
    ['City center at {p0} permanently destroyed; city limit reduced to {p1}', ['C1', '6']],
    ['{p0} at {p1} permanently destroyed; total piece limit reduced to {p2}', ['Temple', 'C2', '4']],
    ['Razed {p0} because no city centers were available', ['C1']],
    ['Obelisk total piece limit reduced to {p0}', ['4']],
  ];
  const directory = new URL('./localization/', import.meta.url);
  for (const file of readdirSync(directory).filter((n) => n.endsWith('.json'))) {
    const locale = file.slice(0, -5);
    const catalog = JSON.parse(readFileSync(new URL(file, directory), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    for (const [template, values] of samples) {
      const source = template.replace(/\{p(\d+)\}/g, (_, i) => values[Number(i)]);
      const expected = catalog[template].replace(/\{p(\d+)\}/g, (_: string, i: string) =>
        translator.translate(values[Number(i)]),
      );
      assert.equal(translator.translate(source), expected, `${locale}: ${source}`);
      const parts = localizedParts(source, journalParts(source), translator.translate);
      assert.equal(parts.map((p) => p.text).join(''), expected, locale);
    }
  }
});
