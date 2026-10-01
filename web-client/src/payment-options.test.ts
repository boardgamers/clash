import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Pile } from './types.ts';
import { paymentWithAmount, samePayment } from './payment-options.ts';

test('payment changes preserve fixed costs and prefer gold substitutions', () => {
  const initial = { food: 3, wood: 4, ore: 5, culture_tokens: 5 };
  const choices: Pile[] = [
    initial,
    { food: 2, wood: 4, ore: 5, culture_tokens: 5, gold: 1 },
    { food: 2, wood: 3, ore: 5, culture_tokens: 5, gold: 2 },
    { food: 2, wood: 4, ore: 4, culture_tokens: 5, gold: 2 },
  ];
  assert.deepEqual(paymentWithAmount(choices, initial, 'food', 2), choices[1]);
  for (const resource of ['food', 'wood', 'ore', 'gold'] as const)
    for (let amount = 0; amount <= 7; amount++) {
      const selected = paymentWithAmount(choices, initial, resource, amount);
      assert(choices.includes(selected), 'Amounts can only produce a complete approved option');
      assert.equal(selected.culture_tokens, 5);
    }
  assert(samePayment({ food: 2 }, { food: 2, gold: 0 }));
  assert(!samePayment({ food: 2 }, { food: 1, gold: 1 }));
});
