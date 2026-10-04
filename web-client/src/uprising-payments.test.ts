import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { Pile, View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'uprising-payments', {})).players.slice(2);
function pending(resources: Pile) {
  // A saved, already-open event must acquire the corrected choices on reload.
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/incidents/civil_war/uprising.outcome.json', import.meta.url),
      'utf8',
    ),
  );
  for (const p of npcs)
    if (!g.players.some((q: any) => q.civilization === p.civilization))
      g.players.push({ ...p, id: g.players.length });
  g.players[0].resources = resources;
  return JSON.stringify(g);
}
const view = (state: string): View => JSON.parse(engine.webView(engine.stripSecret(state, 0), 0));
const quote = (state: string, payment: Pile) =>
  JSON.parse(
    engine.webQuery(
      engine.stripSecret(state, 0),
      0,
      JSON.stringify({ kind: 'decision', values: [], payments: [payment] }),
    ),
  );

test('Uprising offers both culture tokens and awards half a point per token actually paid', () => {
  const state = pending({ culture_tokens: 2 });
  assert.deepEqual(view(state).decision!.fields[0].choices, [{ culture_tokens: 1 }, { culture_tokens: 2 }]);
  for (const amount of [1, 2]) {
    const { action } = quote(state, { culture_tokens: amount });
    assert.deepEqual(action, { Response: { Payment: [{ culture_tokens: amount }] } });
    const after = engine.tryMove(state, JSON.stringify(action), 0);
    assert.equal(JSON.parse(after).players[0].resources?.culture_tokens ?? 0, 2 - amount);
    assert.equal(view(after).players[0].score - view(state).players[0].score, amount / 2);
    const undone = engine.tryMove(after, JSON.stringify('Undo'), 0);
    assert.deepEqual(view(undone).decision!.fields[0].choices, view(state).decision!.fields[0].choices);
  }
  assert.throws(() => quote(state, { culture_tokens: 3 }), /resources/);
});

test('Uprising allows every affordable mood/culture mix totaling 1–4, never zero or five', () => {
  for (const stock of [
    { mood_tokens: 4, culture_tokens: 4 },
    { mood_tokens: 1, culture_tokens: 2 },
  ]) {
    const state = pending(stock);
    const choices = view(state).decision!.fields[0].choices!;
    const expected: Pile[] = [];
    for (let mood = 0; mood <= stock.mood_tokens; mood++)
      for (let culture = 0; culture <= stock.culture_tokens; culture++)
        if (mood + culture >= 1 && mood + culture <= 4)
          expected.push({
            ...(culture ? { culture_tokens: culture } : {}),
            ...(mood ? { mood_tokens: mood } : {}),
          });
    const normalized = (piles: Pile[]) =>
      piles.map((p) => `${p.mood_tokens ?? 0},${p.culture_tokens ?? 0}`).sort();
    assert.deepEqual(normalized(choices), normalized(expected));
    for (const payment of expected) {
      const { action } = quote(state, payment);
      const after = engine.tryMove(state, JSON.stringify(action), 0);
      assert.equal(
        view(after).players[0].score - view(state).players[0].score,
        ((payment.mood_tokens ?? 0) + (payment.culture_tokens ?? 0)) / 2,
      );
    }
  }
  const rich = pending({ mood_tokens: 5, culture_tokens: 5, gold: 5 });
  for (const payment of [
    {},
    { mood_tokens: 5 },
    { culture_tokens: 5 },
    { mood_tokens: 3, culture_tokens: 2 },
    { gold: 2 },
  ])
    assert.throws(() => quote(rich, payment), /valid payment/);
});
