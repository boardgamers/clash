import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { greatDiscovererFixture } from '../scripts/great-discoverer-fixture.mjs';
import { createTranslator, languages } from './localization/runtime.js';
import { printedCardName } from './card-names.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const rule = 'Then, you may explore a region adjacent to a region where you have a city. If you do, you may pay 2 food to found a city in the explored region, without using a Settler.';

for (const [civilization, irrigation, barren] of [['Egypt', true, true], ['Egypt', false, false], ['Rome', true, false]] as const) {
  test(`Great Discoverer: ${civilization}, Irrigation ${irrigation}, Barren ${barren} in real engine and webView`, () => {
    const f = greatDiscovererFixture(engine, { civilization, irrigation });
    f.reveal();
    const choices = f.request().SelectPositions.choices;
    assert.equal(choices.includes('B7'), barren);
    assert.deepEqual(f.view().decision.options.map((o: any) => o.position), choices);
    assert.equal(f.view().decision.min, 0, 'placement remains optional');
    const target = barren ? 'B7' : 'B6';
    const before = JSON.parse(f.state).players[1];
    f.apply({ Response: { SelectPositions: [target] } });
    assert.equal(f.request().Payment[0].optional, false);
    assert.equal(JSON.parse(f.state).players[1].cities.length, before.cities.length);
    f.apply({ Response: { Payment: [{ food: 2 }] } });
    const after = JSON.parse(f.state).players[1];
    assert.equal(after.cities.at(-1).position, target);
    assert.equal(after.resources.food ?? 0, 0);
    assert.deepEqual(after.units, before.units, 'no Settler consumed');
  });
}

for (const resources of [{ food: 1, gold: 1 }, { gold: 2 }]) {
  test(`Great Discoverer: exact gold substitution ${JSON.stringify(resources)}`, () => {
    const f = greatDiscovererFixture(engine, { resources });
    f.reveal();
    f.apply({ Response: { SelectPositions: ['B7'] } });
    const pending = f.state;
    for (const payment of [{}, { food: 1 }, { gold: 1 }, { wood: 2 }, { gold: 3 }]) {
      assert.throws(() => f.apply({ Response: { Payment: [payment] } }));
      assert.equal(f.state, pending, 'invalid payments cannot modify saved state');
    }
    f.apply({ Response: { Payment: [resources] } });
    assert.deepEqual(JSON.parse(f.state).players[1].resources, { mood_tokens: 1 });
    assert.equal(JSON.parse(f.state).players[1].cities.length, 2);
  });
}

for (const noCities of [false, true]) {
  test(`Great Discoverer: ${noCities ? 'no supply' : '0 food and 1 gold'} skips placement, not exploration or free advance`, () => {
    const f = greatDiscovererFixture(engine, { resources: noCities ? { food: 2 } : { gold: 1 }, noCities });
    const before = f.reveal();
    const after = JSON.parse(f.state);
    assert.equal(f.view().decision, null);
    assert.equal(f.request(), undefined);
    assert.ok(after.players[1].advances.includes('Fishing'));
    assert.deepEqual(after.players[1].resources, before.players[1].resources);
    assert.deepEqual(after.players[1].cities, before.players[1].cities);
    assert.equal(after.map.tiles.find(([p]: [string]) => p === 'B7')[1], 'Barren');
    assert.ok(f.state.includes(noCities ? 'You have reached your city limit' : 'Player cannot afford to build a city'));
    assert.ok(!f.state.includes('Decided to build a city'));
  });
}

test('Great Discoverer: decline is free and excluded sites cannot be submitted', () => {
  const f = greatDiscovererFixture(engine, { mixedTerrain: true });
  f.reveal();
  assert.deepEqual(f.request().SelectPositions.choices, ['B6', 'B7']);
  for (const pos of ['A7', 'C7', 'D7']) assert.throws(() => f.apply({ Response: { SelectPositions: [pos] } }));
  const before = JSON.parse(f.state).players[1];
  f.apply({ Response: { SelectPositions: [] } });
  const after = JSON.parse(f.state).players[1];
  assert.deepEqual(after.resources, before.resources);
  assert.deepEqual(after.cities, before.cities);
  assert.equal(f.view().decision, null);
});

test('Great Discoverer: saved legacy position is revalidated before any decided-to-build log', () => {
  const f = greatDiscovererFixture(engine);
  f.reveal();
  const saved = JSON.parse(f.state);
  saved.players[1].resources = { gold: 1 };
  f.state = JSON.stringify(saved);
  f.apply({ Response: { SelectPositions: ['B6'] } });
  assert.equal(f.view().decision, null);
  assert.deepEqual(JSON.parse(f.state).players[1].resources, { gold: 1 });
  assert.equal(JSON.parse(f.state).players[1].cities.length, 1);
  assert.ok(f.state.includes('Player cannot afford to build a city'));
  assert.ok(!f.state.includes('Decided to build a city'));
});

test('Great Discoverer: printed E18 cost and all 17 translated rule catalogs', () => {
  const f = greatDiscovererFixture(engine);
  const card = f.view().actionCards.find((c: any) => c.id === 118);
  assert.equal(printedCardName(card.name), 'Great Discoverer');
  assert.ok(card.description.endsWith(rule), card.description);
  for (const locale of Object.keys(languages)) {
    const catalog = JSON.parse(readFileSync(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    for (const key of [rule, `{p0} ${rule}`]) {
      assert.ok(Object.hasOwn(catalog, key), `${locale}: complete rule exists`);
      if (locale !== 'en') assert.notEqual(catalog[key], key, `${locale}: not English fallback`);
    }
    assert.equal(translator.translate(rule), catalog[rule]);
    assert.ok(translator.translate(card.description).endsWith(catalog[rule]), `${locale}: full composed card translates cost`);
  }
});
