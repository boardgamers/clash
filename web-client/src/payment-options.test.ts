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
