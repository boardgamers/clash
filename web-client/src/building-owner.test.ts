import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildingCivilization,
  cityInfluencers,
  cityInfluenceSummary,
  emblemInkColor,
  foreignBuildingCivilization,
} from './building-owner.ts';

const rome = { id: 4, civilization: 'Rome' };
const greece = { id: 7, civilization: 'Greece' };
const egypt = { id: 9, civilization: 'Egypt' };
const barbarians = { id: 12, civilization: 'Barbarians' };
const players = [rome, greece, egypt, barbarians];

test('piece ownership uses player IDs, not viewer seat or array index', () => {
  assert.equal(buildingCivilization(7, rome, players), 'Greece');
  assert.equal(buildingCivilization(4, rome, players), 'Rome');
  assert.equal(buildingCivilization(12, rome, players), 'Barbarians');
  assert.equal(foreignBuildingCivilization(7, rome, players), 'Greece');
  assert.equal(foreignBuildingCivilization(4, rome, players), undefined);
});

test('missing and invalid owners safely retain host architecture without invented hover ownership', () => {
  for (const owner of [undefined, null, -1, 99, '7', NaN]) {
    assert.equal(buildingCivilization(owner, rome, players), 'Rome');
    assert.equal(foreignBuildingCivilization(owner, rome, players), undefined);
  }
});

test('cityInfluencers groups foreign buildings by owner and ignores host buildings, wonders, and invalid IDs', () => {
  assert.deepEqual(
    cityInfluencers({ city_pieces: { temple: 4, fortress: 4, wonders: ['Pyramids'] } }, rome.id, players),
    [],
  );
  assert.deepEqual(
    cityInfluencers(
      { city_pieces: { academy: 7, fortress: 4, port: 7, temple: 7, wonders: ['GreatGardens'] } },
      rome.id,
      players,
    ),
    [{ owner: 7, civilization: 'Greece', count: 3, buildings: ['Academy', 'Port', 'Temple'] }],
  );
  // Multi-civ influence sorts by count desc then player order, regardless of city_pieces key order.
  assert.deepEqual(
    cityInfluencers({ city_pieces: { obelisk: 9, temple: 7, market: 99 } }, rome.id, players),
    [
      { owner: 7, civilization: 'Greece', count: 1, buildings: ['Temple'] },
      { owner: 9, civilization: 'Egypt', count: 1, buildings: ['Obelisk'] },
    ],
  );
  // Barbarian cities with foreign buildings report influencing civilizations cleanly.
  assert.deepEqual(
    cityInfluencers({ city_pieces: { temple: 7, fortress: 12 } }, barbarians.id, players),
    [{ owner: 7, civilization: 'Greece', count: 1, buildings: ['Temple'] }],
  );
  assert.deepEqual(cityInfluencers(undefined, rome.id, players), []);
});

test('cityInfluenceSummary formats single, counted, and multi-civilization influence labels', () => {
  assert.equal(cityInfluenceSummary([]), '');
  assert.equal(
    cityInfluenceSummary([{ civilization: 'Greece', count: 1 }]),
    'Influenced by Greece',
  );
  assert.equal(
    cityInfluenceSummary([{ civilization: 'Greece', count: 3 }]),
    'Influenced by Greece (3)',
  );
  assert.equal(
    cityInfluenceSummary([
      { civilization: 'Greece', count: 1 },
      { civilization: 'Egypt', count: 1 },
    ]),
    'Influenced by Greece, Egypt',
  );
});

test('emblemInkColor matches light and dark contrast thresholds', () => {
  assert.equal(emblemInkColor('#5086af'), '#fff4dc');
  assert.equal(emblemInkColor('#b96c4c'), '#fff4dc');
  assert.equal(emblemInkColor('#6d9173'), '#fff4dc');
  assert.equal(emblemInkColor('#ae92b6'), '#293b37');
  assert.equal(emblemInkColor('#e7d8b5'), '#293b37');
});
