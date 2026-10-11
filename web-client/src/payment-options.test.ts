import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Pile } from './types.ts';
import { paymentWithAmount, samePayment, rewardWithAmount, canCompletePayment } from './payment-options.ts';

test('tax allocations keep other resources fixed and allow only the remaining total', () => {
  const choices: Pile[] = [];
  for (let food = 0; food <= 6; food++)
    for (let wood = 0; wood <= 6 - food; wood++) choices.push({ food, wood, ore: 6 - food - wood });
  const full = { food: 4, ore: 2 };
  assert.deepEqual(rewardWithAmount(choices, full, 'wood', 1), full);
  const partial = rewardWithAmount(choices, full, 'food', 3);
  assert.deepEqual(partial, { food: 3, ore: 2 });
  assert(canCompletePayment(choices, partial));
  assert(
    !choices.some((choice) => samePayment(choice, partial)),
    'partial selection is not ready to confirm',
  );
  assert.deepEqual(rewardWithAmount(choices, partial, 'wood', 2), partial);
  const completed = rewardWithAmount(choices, partial, 'wood', 1);
  assert.deepEqual(completed, { food: 3, ore: 2, wood: 1 });
  assert(choices.some((choice) => samePayment(choice, completed)));
  assert(
    !canCompletePayment([{ mood_tokens: 2 }, { culture_tokens: 2 }], { mood_tokens: 1, culture_tokens: 1 }),
    'restricted combinations remain restricted',
  );
});

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


test('Trade Routes swaps the requested reward directly, reducing food first', () => {
  const choices: Pile[] = [];
  for (let food = 0; food <= 3; food++)
    for (let gold = 0; gold <= 3 - food; gold++) choices.push({ food, gold, mood_tokens: 3 - food - gold });
  const next = paymentWithAmount(choices, { food: 2, gold: 1 }, 'mood_tokens', 2, 'food');
  assert.deepEqual(next, { food: 0, gold: 1, mood_tokens: 2 });
  assert.deepEqual(paymentWithAmount(choices, next, 'mood_tokens', 0, 'food'), { food: 2, gold: 1, mood_tokens: 0 });
  for (const resource of ['food', 'gold', 'mood_tokens'] as const)
    for (let amount = 0; amount <= 3; amount++) {
      const changed = paymentWithAmount(choices, next, resource, amount, 'food');
      assert.equal(changed[resource], amount);
      assert.equal(Object.values(changed).reduce((a, b) => a + (b ?? 0), 0), 3);
      assert(choices.includes(changed));
    }
  const limited = [{ food: 2 }, { food: 1, culture_tokens: 1 }, { food: 1, ideas: 2 }];
  assert.deepEqual(paymentWithAmount(limited, limited[0], 'ideas', 2, 'food'), limited[2]);
  assert.deepEqual(paymentWithAmount(limited, limited[0], 'culture_tokens', 2, 'food'), limited[0]);
});
