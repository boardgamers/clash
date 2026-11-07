import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { toggleDecisionSelection } from './decision-controls.ts';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

for (const id of [31, 32]) test(`New Goals ${id}: explicit private exchange preserves engine rules`, async () => {
  const game = JSON.parse(await engine.init(2, [], {}, `new-goals-${id}`, {}));
  const seat = engine.currentPlayer(JSON.stringify(game));
  game.players[seat].action_cards = [id];
  game.players[seat].resources.culture_tokens = 5;
  // A second existing objective exercises replacing the return selection too.
  game.players[seat].objective_cards.push(game.objective_cards_left.pop());
  const before = [...game.players[seat].objective_cards];
  const move = (state: string, action: unknown) => engine.tryMove(state, JSON.stringify(action), seat);
  let state = move(JSON.stringify(game), { Playing: { ActionCard: id } });
  state = move(state, { Response: { Payment: [{ culture_tokens: 1 }] } });
  const visible = engine.stripSecret(state, seat);
  const decision = (JSON.parse(engine.webView(visible, seat)) as View).decision!;
  assert.equal(decision.objectiveExchange, true);
  const keeps = decision.options.flatMap((o, i) => o.exchangeRole === 'keep' ? [i] : []);
  const returns = decision.options.flatMap((o, i) => o.exchangeRole === 'return' ? [i] : []);
  assert.equal(keeps.length, 2);
  assert.equal(returns.length, before.length);
  assert.ok(decision.options.every(o => o.card?.kind === 'objective' && o.card.objectives.length === 2));
  const query = (indices: number[]) => JSON.parse(engine.webQuery(visible, seat, JSON.stringify({kind: 'decision', values: indices.map(i => decision.options[i].value), payments: []})));
  for (const indices of [[keeps[0]], keeps, returns, [keeps[0], keeps[0]], [keeps[0], ...returns]]) assert.throws(() => query(indices));
  let selected = toggleDecisionSelection(decision, [], keeps[0]);
  selected = toggleDecisionSelection(decision, selected, keeps[1]);
  assert.deepEqual(selected, [keeps[1]]);
  selected = toggleDecisionSelection(decision, selected, returns[0]);
  selected = toggleDecisionSelection(decision, selected, returns[1]);
  assert.deepEqual(selected, [keeps[1], returns[1]]);
  assert.deepEqual(toggleDecisionSelection(decision, selected, returns[1]), [keeps[1]]);
  const kept = JSON.parse(move(state, query([]).action));
  assert.deepEqual(kept.players[seat].objective_cards, before);
  const exchanged = JSON.parse(move(state, query(selected).action));
  const gained = (decision.options[keeps[1]].value as {ObjectiveCard: number}).ObjectiveCard;
  const returned = (decision.options[returns[1]].value as {ObjectiveCard: number}).ObjectiveCard;
  assert.deepEqual([...exchanged.players[seat].objective_cards].sort(), [...before.filter(c => c !== returned), gained].sort());
  for (const after of [kept, exchanged]) {
    assert.equal(after.players[seat].resources.culture_tokens, 4);
    assert.equal(after.actions_left, game.actions_left - 1);
    assert.ok(!(after.players[seat].action_cards ?? []).includes(id));
  }
  const other = 1 - seat;
  assert.equal(JSON.parse(engine.webView(engine.stripSecret(state, other), other)).decision, null);
  assert.ok(JSON.parse(engine.stripSecret(state, other)).players[seat].objective_cards.every((c: number) => c === 0));
});
