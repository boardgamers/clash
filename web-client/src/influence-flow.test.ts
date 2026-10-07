import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import {
  activeInfluence,
  influenceKey,
  influenceMapPick,
  influencePaymentMatches,
  influenceTarget,
  influenceUpfrontCost,
} from './influence.ts';
import type { Session } from './types.ts';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'influence-npcs', {})).players.slice(2);
export function influenceFixture() {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/base/cultural_influence_instant.json', import.meta.url),
      'utf8',
    ),
  );
  for (const npc of npcs)
    if (!g.players.some((p: any) => p.civilization === npc.civilization))
      g.players.push({ ...npc, id: g.players.length });
  g.dice_roll_outcomes = [0];
  g.current_player_index = 0;
  g.players[1].cities[0].city_pieces = { temple: 1 };
  g.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
  g.players[0].civilization = 'Rome';
  g.players[0].resources.culture_tokens = 7;
  return g;
}
const view = (g: any): View => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const move = (g: any, action: any) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));

test('influence exposes upfront fees and preserves target, source and roll across range and boost choices', () => {
  const g = influenceFixture();
  const target = view(g).influence!.find((t) => t.position === 'C1' && t.name === 'Temple')!;
  const origin = target.origins!.find((o) => o.position === target.origin)!;
  assert.equal(origin.free, false);
  assert.deepEqual(origin.actionPayment, {});
  assert.ok(origin.payment.culture_tokens);
  assert.deepEqual(influenceUpfrontCost({ culture_tokens: 1 }, origin.payment), {
    culture_tokens: 1 + origin.payment.culture_tokens!,
  });
  const range = move(g, origin.action);
  const v = view(range);
  assert.equal(v.influenceContext!.stage, 'range');
  assert.equal(v.influenceContext!.target, 'C1');
  assert.equal(v.influenceContext!.source, origin.position);
  assert.equal(v.influenceContext!.roll, null, 'the roll is hidden before paying for range');
  assert.ok(influencePaymentMatches(v, origin.payment));
  assert.ok(!influencePaymentMatches(v, { culture_tokens: 99 }));
  const boosted = move(range, { Response: { Payment: [origin.payment] } });
  const result = view(boosted);
  assert.equal(result.influenceContext!.stage, 'boost');
  assert.ok(result.influenceContext!.roll! < 5);
  assert.equal(result.influenceContext!.target, 'C1');
  assert.ok(
    !influencePaymentMatches(result, result.decision!.fields[0].cost),
    'an optional boost is never an accepted range payment',
  );
  const declined = move(boosted, { Response: { Payment: [{}] } });
  assert.equal(view(declined).influenceContext, null);
  assert.ok(!declined.successful_cultural_influence);
});

test('Buddhism keeps the same influence context through an optional reroll', () => {
  const g = influenceFixture(),
    p = g.players[0];
  p.civilization = 'India';
  p.advances = [...new Set([...p.advances, 'Myths', 'StateReligion'])];
  p.cities[0].city_pieces = { temple: 0 };
  p.units = [{ id: 0, position: 'A1', unit_type: { Leader: 'Ashoka' } }];
  p.next_unit_id = 1;
  g.dice_roll_outcomes = [10, 0];
  const offer = view(g).influence!.find((t) => t.position === 'C1' && t.name === 'Temple')!;
  const origin = offer.origins!.find((o) => o.position === 'A1')!;
  assert.ok(origin.reroll);
  const rolled = move(g, origin.action);
  assert.equal(view(rolled).influenceContext!.stage, 'reroll');
  assert.equal(view(rolled).influenceContext!.roll, 1 + (origin.rollBonus ?? 0));
  assert.ok(view(rolled).choiceDecision);
  assert.ok(!influencePaymentMatches(view(rolled), {}));
  const kept = move(rolled, { Response: { Bool: false } });
  assert.equal(view(kept).influenceContext!.stage, 'boost');
  const rerolled = move(rolled, { Response: { Bool: true } });
  assert.equal(view(rerolled).influenceContext, null);
  assert.ok(rerolled.successful_cultural_influence);
});

test('influence map clicks choose the exact clicked building and only auto-pick a lone candidate', () => {
  const g = influenceFixture();
  g.players[1].cities[0].city_pieces = { temple: 1, academy: 1 };
  const v = view(g);
  const offers = v.influence!;
  const temple = offers.find((o) => o.name === 'Temple')!;
  assert.deepEqual(influenceTarget(temple), { kind: 'structure', structure: 'Building:Temple' });
  assert.equal(influenceMapPick(offers, 'C1', { kind: 'city', structure: 'Building:Temple' }), temple);
  assert.equal(influenceMapPick(offers, 'C1', { kind: 'city' }), null, 'two buildings: the city opens a list');
  assert.equal(influenceMapPick(offers, 'C1', { kind: 'city', structure: 'CityCenter' }), null);
  assert.equal(influenceMapPick(offers, 'A1', { kind: 'city' }), null);
  const lone = view(influenceFixture()).influence!;
  assert.equal(influenceMapPick(lone, 'C1', { kind: 'tile' })?.name, 'Temple');
  const s = (patch: Partial<Session>) =>
    ({ view: v, mode: 'overview', abilitiesOpen: true, influenceMode: true, ...patch }) as Session;
  assert.equal(activeInfluence(s({ influenceMode: false })), undefined);
  assert.equal(activeInfluence(s({ abilitiesOpen: false })), undefined);
  const browsing = activeInfluence(s({ influencePosition: 'C1' }))!;
  assert.deepEqual(browsing.targets, ['C1']);
  assert.equal(browsing.target, null);
  assert.deepEqual(browsing.selected, ['C1']);
  const chosen = activeInfluence(s({ influenceTarget: influenceKey(temple) }))!;
  assert.equal(chosen.target, temple);
  assert.equal(chosen.source, 'A1');
  assert.deepEqual(chosen.selected, ['C1', 'A1']);
  assert.deepEqual(chosen.positions, ['C1', 'A1'], 'the source city is highlighted with the targets');
});
