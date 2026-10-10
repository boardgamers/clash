import { test } from 'node:test';
import assert from 'node:assert/strict';
import { effectSourceReference } from './active-effects.ts';
import type { ActiveEffect, EventInfo } from './types.ts';
import type { CardRule } from './card-reference.ts';

test('effects open the actual source event or action card using stable references', () => {
  const trojan = { id: 42, name: 'Trojan Horse', rules: ['Printed rules'] } as EventInfo;
  const card = {
    id: 29,
    name: 'Mass Production',
    description: 'Two additional tiles',
    tactics: { name: 'Defensive Formation', description: 'Battle use' },
  } as CardRule;
  const effect = { source: { kind: 'Event', name: 'Trojan Horse', id: 42 } } as ActiveEffect;
  assert.deepEqual(effectSourceReference(effect, [trojan], [card]), { kind: 'event', event: trojan });
  assert.deepEqual(
    effectSourceReference(
      { ...effect, source: { kind: 'Action card', name: 'Mass Production' } },
      [trojan],
      [card],
    ),
    { kind: 'card', card },
  );
  assert.equal(effectSourceReference(effect, [], [card]), null);
  assert.equal(effectSourceReference({ ...effect, source: undefined }, [trojan], [card]), null);
});

test('action effect references distinguish copies with different battle uses', () => {
  const cards = [
    { id: 29, name: 'Mass Production', tactics: { name: 'Defensive Formation' } },
    { id: 30, name: 'Mass Production', tactics: { name: 'Encircled' } },
  ] as CardRule[];
  const effect = { source: { kind: 'Action card', name: 'Mass Production', id: 30 } } as ActiveEffect;
  assert.equal(effectSourceReference(effect, [], cards)?.kind, 'card');
  const exact = effectSourceReference(effect, [], cards);
  assert.equal(exact?.kind === 'card' && exact.card.id, 30);
  const legacy = effectSourceReference(
    { ...effect, source: { kind: 'Action card', name: 'Mass Production' } },
    [],
    cards,
  );
  assert.equal(legacy?.kind === 'card' && legacy.card.tactics, null);
});
