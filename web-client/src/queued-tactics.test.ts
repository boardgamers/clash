import assert from 'node:assert/strict';
import test from 'node:test';
import { queuedTacticsDecision, type TacticsQueue } from './queued-tactics.ts';
import type { ActionCard, View } from './types.ts';
const card = (id: number): ActionCard => ({
  id,
  name: 'Civil side',
  description: 'Civil rules',
  free: false,
  cost: {},
  tactics: { name: 'Routing', description: 'Battle rules' },
  reason: null,
  action: null,
});
const view = { actionCards: [card(3), card(0)] } as View;
const offer: TacticsQueue = {
  key: 'battle/tactics',
  title: 'Tactics',
  selected: null,
  options: [
    [],
    [{ ActionCard: 3 }],
    [{ ActionCard: 4 }],
    [{ ActionCard: 0 }],
    [{ ActionCard: 3 }, { ActionCard: 4 }],
  ].map((cards) => ({ label: 'offer', description: '', response: { SelectHandCards: cards } })),
};
test('queued tactics uses only single authorized cards in the owner hand, never hidden placeholders', () => {
  const decision = queuedTacticsDecision(view, offer)!;
  assert.equal(decision.tacticsSelection, true);
  assert.equal(decision.min, 0);
  assert.equal(decision.max, 1);
  assert.equal(decision.options.length, 1);
  assert.deepEqual(decision.options[0].value, { ActionCard: 3 });
  assert.equal(decision.options[0].card?.kind, 'action');
  assert.deepEqual(
    decision.options[0].card?.kind === 'action' && decision.options[0].card.tactics,
    card(3).tactics,
  );
  assert.equal(queuedTacticsDecision(null, offer), null);
  assert.equal(queuedTacticsDecision(view, undefined), null);
});
