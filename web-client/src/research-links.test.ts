import { test } from 'node:test';
import assert from 'node:assert/strict';
import { manGodAbility, researchReferences, researchTextParts, researchOwners, wonderResearchHints } from './research-links.ts';
import type { AdvanceView, View } from './types.ts';

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

test('research highlights only missing advances needed by wonder cards in the viewing hand', () => {
  const library = { id: 'GreatLibrary', name: 'Great Library', requiredAdvance: 'Philosophy', requiredAdvanceOwned: false };
  const garden = { id: 'GreatGardens', name: 'Great Gardens', requiredAdvance: 'Irrigation', requiredAdvanceOwned: false };
  const view = { wonderCards: [library, garden] } as View;
  const philosophy = { id: 'Philosophy', name: 'Philosophy', owned: false } as AdvanceView;
  assert.deepEqual(wonderResearchHints(view, philosophy), [library]);
  assert.deepEqual(wonderResearchHints(view, { ...philosophy, name: 'Education' }), []);
  assert.deepEqual(wonderResearchHints(view, { ...philosophy, owned: true }), []);
  assert.deepEqual(wonderResearchHints({ wonderCards: [{ ...library, requiredAdvanceOwned: true }] } as View, philosophy), []);
  assert.deepEqual(wonderResearchHints({ wonderCards: [] } as unknown as View, philosophy), []);
  assert.deepEqual(wonderResearchHints(null, philosophy), []);
});

test('Man God indicator distinguishes granted abilities from research and Great Library loans', () => {
  const advance = { id: 'Nationalism', owned: false };
  const abilities = [{ ...advances[0], id: 'Nationalism', borrowed: true, borrowedSource: 'Man God', description: '', group: 'Autocracy', order: 0 }];
  assert.equal(manGodAbility(advance, abilities), true);
  assert.equal(manGodAbility({ ...advance, owned: true }, abilities), false);
  assert.equal(manGodAbility(advance), false);
  assert.equal(manGodAbility({ id: 'Totalitarianism', owned: false }, abilities), false);
  assert.equal(manGodAbility(advance, [{ ...abilities[0], borrowedSource: 'Great Library' }]), false);
  assert.equal(manGodAbility(advance, [{ ...abilities[0], borrowed: false }]), false);
});
