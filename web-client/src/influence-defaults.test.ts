import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { activeInfluence, bestInfluenceOrigin, cheaperInfluenceOrigin, influenceKey, influenceSuccessScore, type InfluenceOffer } from './influence.ts';
import type { Session, View } from './types.ts';

type Origin = NonNullable<InfluenceOffer['origins']>[number];
const source = (position: string, patch: Partial<Origin> = {}): Origin => ({
  position, settlers: false, reroll: false, payment: {}, action: {}, free: false,
  actionPayment: {}, rollBonus: 0, preventBoost: false, ...patch,
});
const offer = (...origins: Origin[]): InfluenceOffer => ({
  name: 'Market', position: 'C1', origin: origins[0].position, origins,
  variant: 'Influence Culture', payment: {}, action: {},
});
const unity = [{ name: 'Cultural Unity', resources: { mood_tokens: 1 } }];

test('equal-cost defaults compare exact free success chances, not just reroll flags', () => {
  const ordinary = source('A1'), reroll = source('C2', { reroll: true });
  assert.equal(influenceSuccessScore(ordinary), 12);
  assert.equal(influenceSuccessScore(reroll), 20);
  assert.equal(influenceSuccessScore(source('C2', { rollBonus: 1 })), 18);
  assert.equal(influenceSuccessScore(source('C2', { rollBonus: 1, reroll: true })), 27);
  assert.equal(bestInfluenceOrigin(offer(ordinary, source('C2', { rollBonus: 1 })))?.position, 'C2');
  assert.equal(bestInfluenceOrigin(offer(source('A1', { rollBonus: 1 }), reroll))?.position, 'C2');
  assert.equal(bestInfluenceOrigin(offer(reroll, source('A1', { rollBonus: 2 })))?.position, 'A1');
  const guaranteed = source('A1', { rollBonus: 4 });
  assert.equal(influenceSuccessScore(guaranteed), 36);
  assert.equal(influenceSuccessScore(source('C2', { rollBonus: 8, reroll: true })), 36);
  assert.equal(bestInfluenceOrigin(offer(guaranteed, source('C2', { rollBonus: 4, reroll: true })))?.position, 'A1', 'do not value an unusable reroll');
});

test('equal chances prefer boosting and source rewards without overriding costs or manual choices', () => {
  const ordinary = source('A1'), darius = source('C2', { successRewards: unity });
  assert.equal(bestInfluenceOrigin(offer(ordinary, darius))?.position, 'C2');
  assert.equal(bestInfluenceOrigin(offer(darius, ordinary))?.position, 'C2');
  assert.equal(bestInfluenceOrigin(offer(source('A1', { preventBoost: true }), source('C2')))?.position, 'C2');
  assert.equal(bestInfluenceOrigin(offer(ordinary, { ...darius, payment: { culture_tokens: 1 } }))?.position, 'A1');
  assert.equal(bestInfluenceOrigin(offer(ordinary, { ...darius, actionPayment: { mood_tokens: 1 } }))?.position, 'A1');
  assert.equal(bestInfluenceOrigin(offer({ ...ordinary, free: true }, darius))?.position, 'A1');
  assert.equal(bestInfluenceOrigin(offer(ordinary, { ...darius, rollBonus: 1, payment: { culture_tokens: 1 } }))?.position, 'A1');
  assert.equal(bestInfluenceOrigin(offer(ordinary, source('C2')))?.position, 'A1', 'stable fallback');
  const target = offer(ordinary, darius);
  const s = { mode: 'overview', abilitiesOpen: true, influenceMode: true,
    influenceTarget: influenceKey(target), influenceOrigin: 'A1',
    view: { canPlay: true, influence: [target] } } as Session;
  assert.equal(activeInfluence(s)?.source, 'A1');
  assert.deepEqual(target.origins, [ordinary, darius], 'sorting does not mutate engine offers');
});

test('cheaper-source suggestions never silently trade away success chances, boosting or rewards', () => {
  const cheaper = source('A1'), remote = source('C2', { payment: { culture_tokens: 1 } });
  for (const benefit of [{ reroll: true }, { rollBonus: 1 }, { successRewards: unity }]) {
    const selected = { ...remote, ...benefit };
    assert.equal(cheaperInfluenceOrigin(offer(cheaper, selected), selected), null);
    const equivalent = { ...cheaper, ...benefit };
    assert.equal(cheaperInfluenceOrigin(offer(equivalent, selected), selected), equivalent);
  }
  assert.equal(cheaperInfluenceOrigin(offer({ ...cheaper, preventBoost: true }, remote), remote), null);
  assert.equal(cheaperInfluenceOrigin(offer(cheaper, remote), remote), cheaper);
});

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'influence-defaults', {})).players.slice(2);
const view = (g: any): View => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const move = (g: any, action: unknown) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
function dariusFixture() {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/base/cultural_influence_instant.json', import.meta.url), 'utf8'));
  g.players = [...g.players.slice(0, 2), ...structuredClone(npcs)];
  g.current_player_index = 0;
  g.dice_roll_outcomes = [10];
  const p = g.players[0];
  p.civilization = 'Persia';
  p.advances.push('Arts', 'Myths', 'Priesthood');
  p.cities = ['A1', 'C2'].map(position => ({ position, mood_state: 'Happy', city_pieces: { temple: 0, academy: 0 } }));
  p.units = [{ id: 0, position: 'C2', unit_type: { Leader: 'Darius' } }];
  p.next_unit_id = 1;
  p.resources.culture_tokens = 7;
  p.resources.mood_tokens = 12; // Mood tokens are not capped at the ordinary resource limit.
  g.players[1].cities[0].city_pieces = { market: 1 };
  g.players[1].units = [{ id: 0, position: 'B3', unit_type: 'Infantry' }];
  g.players[1].next_unit_id = 1;
  return g;
}
function resolveAttempt(g: any, from: Origin) {
  let next = move(g, from.action);
  if (view(next).influenceContext?.stage === 'payment') next = move(next, { Response: { Payment: [from.actionPayment] } });
  if (view(next).influenceContext?.stage === 'range') next = move(next, { Response: { Payment: [from.payment] } });
  return next;
}

test('real Darius defaults include Arts and unit conversion, and match the actual source reward', () => {
  const g = dariusFixture(), before = JSON.stringify(g);
  const targets = view(g).influence!;
  assert(targets.some(t => t.variant === 'Arts'));
  assert(targets.some(t => t.variant === 'Zoroastrianism'));
  for (const target of targets) {
    const a = target.origins!.find(o => o.position === 'A1')!;
    const d = target.origins!.find(o => o.position === 'C2')!;
    assert(a && d);
    assert.deepEqual(d.payment, a.payment);
    assert.deepEqual(a.successRewards, []);
    assert.deepEqual(d.successRewards, unity);
    assert.equal(bestInfluenceOrigin(target)?.position, 'C2');
    for (const from of [a, d]) {
      const after = resolveAttempt(g, from);
      assert.equal(after.successful_cultural_influence, true);
      const actionMood = from.actionPayment?.mood_tokens ?? 0;
      assert.equal(after.players[0].resources.mood_tokens, 12 - actionMood + (from === d ? 1 : 0));
    }
  }
  assert.equal(JSON.stringify(g), before, 'previews and sorting do not mutate the state');
});

test('Darius reward metadata follows the actual leader position and never leaks into common success rewards', () => {
  const g = dariusFixture();
  g.players[0].units[0].position = 'A1';
  for (const target of view(g).influence!) {
    assert.deepEqual(target.origins!.find(o => o.position === 'C2')!.successRewards, []);
    assert.equal(bestInfluenceOrigin(target)?.position, 'A1');
  }
  g.players[0].units = [];
  g.players[0].advances.push('Conversion');
  for (const target of view(g).influence!) {
    assert(target.origins!.every(o => o.successRewards?.length === 0));
    assert(target.origins!.every(o => o.rollBonus === 1));
    assert.equal(bestInfluenceOrigin(target)?.position, 'A1');
  }
});
