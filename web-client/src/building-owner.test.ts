import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildingCivilization, foreignBuildingCivilization } from './building-owner.ts';

const rome = { id: 4, civilization: 'Rome' };
const greece = { id: 7, civilization: 'Greece' };
const barbarians = { id: 12, civilization: 'Barbarians' };
const players = [rome, greece, barbarians];
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
