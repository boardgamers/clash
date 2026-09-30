import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { researchDecision, mapDecisionOptions, toggleDecisionSelection } from './decision-controls.ts';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (name: string) =>
  readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8');
const view = (state: string, seat = engine.currentPlayer(state)): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));

test('free research exposes exactly the legal choices without paid actions or resource requirements', () => {
  const raw = JSON.parse(fixture('status_phase/free_advance.outcome'));
  const seat = engine.currentPlayer(JSON.stringify(raw));
  raw.players[seat].resources = {};
  const state = JSON.stringify(raw);
  const v = view(state);
  assert.ok(researchDecision(v));
  assert.deepEqual(
    v.advances
      .filter((a) => a.action)
      .map((a) => a.id)
      .sort(),
    v.decision!.options.map((o) => o.value).sort(),
  );
  const storage = v.advances.find((a) => a.id === 'Storage')!;
  assert.deepEqual(storage.action, { Response: { SelectAdvance: 'Storage' } });
  assert.equal(storage.costAmount, 0);
  assert.deepEqual(storage.payments, []);
  const next = JSON.parse(engine.tryMove(state, JSON.stringify(storage.action), seat));
  assert.ok(next.players[seat].advances.includes('Storage'));
  assert.equal(next.actions_left, raw.actions_left);
  assert.equal(next.players[seat].resources.food ?? 0, 0);
  assert.equal(next.players[seat].resources.gold ?? 0, 0);
  assert.ok(!researchDecision(view(state, 1 - seat)));
  assert.ok(view(state, 1 - seat).advances.every((a) => !a.action));
  assert.ok(!researchDecision(JSON.parse(engine.webView(engine.stripSecret(state)))));
});

test('map position choices select, replace and deselect without submitting until confirmed', () => {
  const raw = JSON.parse(fixture('incidents/barbarians_move'));
  const barbarians = raw.players.find((p: { civilization: string }) => p.civilization === 'Barbarians');
  barbarians.units = [];
  barbarians.cities = [];
  const state = engine.tryMove(
    JSON.stringify(raw),
    JSON.stringify({ Response: { SelectAdvance: 'Storage' } }),
    0,
  );
  const v = view(state);
  const d = v.decision!;
  const options = mapDecisionOptions(d);
  assert.equal(options.length, 2);
  let selected = toggleDecisionSelection(d, [], 0);
  selected = toggleDecisionSelection(d, selected, 1);
  assert.deepEqual(selected, [1]);
  assert.deepEqual(toggleDecisionSelection(d, selected, 1), []);
  assert.deepEqual(toggleDecisionSelection(d, selected, 99), selected);
  const action = JSON.parse(
    engine.webQuery(
      engine.stripSecret(state, 0),
      0,
      JSON.stringify({
        kind: 'decision',
        values: selected.map((i) => options[i].value),
        payments: [],
      }),
    ),
  ).action;
  const next = JSON.parse(engine.tryMove(state, JSON.stringify(action), 0));
  assert.ok(
    next.players
      .find((p: { civilization: string }) => p.civilization === 'Barbarians')
      .cities.some((c: { position: string }) => c.position === options[1].position),
  );
  assert.equal(mapDecisionOptions(view(state, 1).decision).length, 0);
  const multiple = { ...d, max: 2, options: [...options, { ...options[0], value: 'C4', position: 'C4' }] };
  assert.deepEqual(toggleDecisionSelection(multiple, [0, 1], 2), [0, 1]);
  assert.equal(
    mapDecisionOptions({ ...d, options: options.map((o, i) => ({ ...o, value: i })) }).length,
    0,
    'Unit choices must not be confused with tile choices',
  );
});

test('payment choices include optional purchase/decline and complete mixed reward allocations', () => {
  const raw = JSON.parse(fixture('incidents/barbarians_move'));
  raw.incidents_left = [22];
  const barbarians = raw.players.find((p: { civilization: string }) => p.civilization === 'Barbarians');
  barbarians.units = [];
  barbarians.cities = [];
  let state = engine.tryMove(
    JSON.stringify(raw),
    JSON.stringify({ Response: { SelectAdvance: 'Storage' } }),
    0,
  );
  for (let i = 0; i < 8; i++) {
    const d = view(state).decision!;
    if (d.fields.length) break;
    const seat = engine.currentPlayer(state);
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(state, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: d.options.slice(0, d.min).map((o) => o.value),
          payments: [],
        }),
      ),
    ).action;
    state = engine.tryMove(state, JSON.stringify(action), seat);
  }
  const d = view(state).decision!;
  assert.equal(d.name, 'Great Scientist');
  assert.match(d.description, /When played \(1 action\).*Science/);
  assert.deepEqual(d.fields[0].choices, [{ culture_tokens: 1 }, {}]);
  const before = JSON.parse(state);
  for (const payment of d.fields[0].choices!) {
    const seat = engine.currentPlayer(state);
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(state, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: [],
          payments: [payment],
        }),
      ),
    ).action;
    const next = JSON.parse(engine.tryMove(state, JSON.stringify(action), seat));
    if (payment.culture_tokens) {
      assert.ok(next.players[seat].action_cards.includes(122));
      assert.equal(
        next.players[seat].resources.culture_tokens,
        before.players[seat].resources.culture_tokens - 1,
      );
    } else {
      assert.equal(
        next.players[seat].resources.culture_tokens,
        before.players[seat].resources.culture_tokens,
      );
      assert.deepEqual(view(JSON.stringify(next)).decision!.fields[0].choices, [{ culture_tokens: 2 }, {}]);
    }
  }
  const reward = fixture('advances/trade_routes_with_currency.outcome');
  const rewardView = view(reward);
  const choices = rewardView.decision!.fields[0].choices!;
  assert.ok(choices.some((p) => p.food === 1 && p.gold === 1));
  assert.ok(choices.some((p) => p.gold === 2));
  const seat = engine.currentPlayer(reward);
  for (const payment of choices) {
    const action = JSON.parse(
      engine.webQuery(
        engine.stripSecret(reward, seat),
        seat,
        JSON.stringify({
          kind: 'decision',
          values: [],
          payments: [payment],
        }),
      ),
    ).action;
    assert.doesNotThrow(() => engine.tryMove(reward, JSON.stringify(action), seat));
  }
});
