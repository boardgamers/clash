import { test } from 'node:test';
import assert from 'node:assert/strict';
import { researchReferences, researchTextParts, researchOwners } from './research-links.ts';
import type { View } from './types.ts';

const advances = ['Irrigation', 'Education', 'Public Education', 'Mining'].map((name) => ({
  id: name.replaceAll(' ', ''),
  name,
}));

test('journal links only complete advance names and keep surrounding wording unchanged', () => {
  const text = 'Collect with irrigation, Public Education and Education; undermining, Mining, reEducation.';
  const parts = researchTextParts(text, advances);
  assert.equal(parts.map((p) => p.text).join(''), text);
  assert.deepEqual(
    parts.filter((p) => p.research).map((p) => [p.text, p.research?.id]),
    [
      ['irrigation', 'Irrigation'],
      ['Public Education', 'PublicEducation'],
      ['Education', 'Education'],
      ['Mining', 'Mining'],
    ],
  );
  assert.equal(parts[0].text, 'Collect with ');
  assert.deepEqual(researchTextParts('Unknown advance', advances), [{ text: 'Unknown advance' }]);
});

test('repeated and punctuated names remain intact without matching fragments of Unicode words', () => {
  const text = '(Public  Education), IRRIGATION: Irrigation’s effect; Irrigationé.';
  const parts = researchTextParts(text, advances);
  assert.equal(parts.map((p) => p.text).join(''), text);
  assert.deepEqual(
    parts.filter((p) => p.research).map((p) => p.research?.id),
    ['PublicEducation', 'Irrigation', 'Irrigation'],
  );
});

test('civilization unlocks navigate to their journal actor, including spectator views', () => {
  const view = {
    advances: [],
    players: [
      { index: 0, advances: [advances[0]], civilizationAdvances: [{ id: 'Banking', name: 'Banking' }] },
      { index: 1, advances: [advances[2]], civilizationAdvances: [{ id: 'Calendar', name: 'Calendar' }] },
    ],
  } as unknown as View;
  const references = researchReferences(view, 1);
  assert.deepEqual(
    references.find((r) => r.name === 'Calendar'),
    {
      id: 'Calendar',
      name: 'Calendar',
      player: 1,
      civilization: true,
    },
  );
  assert.ok(references.some((r) => r.id === 'Irrigation'));
  assert.ok(references.some((r) => r.id === 'PublicEducation'));
  assert.ok(!references.some((r) => r.id === 'Banking'));
  assert.equal(researchTextParts('Unlock Calendar', references)[1].research?.player, 1);
});

test('research ownership badges use public permanent advances and exclude the viewed player', () => {
  const view = {
    players: [
      { index: 0, advances: [{ id: 'Education' }] },
      { index: 1, advances: [{ id: 'Education' }] },
      { index: 2, advances: [{ id: 'Education', borrowed: true }] },
    ],
  } as unknown as View;
  assert.deepEqual(
    researchOwners(view, 'Education', 0).map((p) => p.index),
    [1],
  );
  assert.deepEqual(
    researchOwners(view, 'Education').map((p) => p.index),
    [0, 1],
  );
  assert.deepEqual(researchOwners(view, 'Mining', 0), []);
});
