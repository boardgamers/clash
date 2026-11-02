import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { researchAbilityHints, researchCardHints } from './research-links.ts';
import { contextualCards } from './contextual-cards.ts';
import type { AdvanceView, View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'research-sources', {})).players.slice(2);
function fixture(name = 'action_cards/synergies') {
  const g = JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
  for (const npc of npcs) if (!g.players.some((p: any) => p.civilization === npc.civilization)) g.players.push({ ...npc, id: g.players.length });
  g.players[0].incident_tokens = 3;
  g.players[0].action_cards = [];
  return g;
}
const view = (g: any, seat: number | null = 0): View => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), seat), seat));
const move = (g: any, action: any) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
const advance = (v: View, id: string) => v.advances.find((a) => a.id === id)!;
const card = (v: View, id: number) => v.actionCards!.find((c) => c.id === id)!;
const targets = (v: View) => v.decision!.options.map((o) => o.value as string).sort();
const roots = ['Farming', 'Mining', 'Fishing', 'Arts', 'Myths', 'Writing', 'Math', 'Tactics', 'Bartering', 'Voting'];

test('research helpers use only usable own-seat metadata, never names or public opponent hands', () => {
  const a = { id: 'Medicine', owned: false } as AdvanceView;
  const source = { id: 122, name: 'localized name', action: { Playing: { ActionCard: 122 } }, cost: {}, freeResearch: true, researchAdvances: ['Medicine'] };
  const v = { canPlay: true, actionCards: [source], specialActions: [{ ...source, id: undefined }] } as unknown as View;
  assert.equal(researchCardHints(v, a).length, 1);
  assert.equal(researchAbilityHints(v, a).length, 1);
  assert.equal(contextualCards(v, 'research')[0].card.id, 122);
  assert.equal(contextualCards(v, 'build').length, 0);
  for (const change of [{ canPlay: false }, { actionCards: [], specialActions: [] }]) {
    assert.deepEqual(researchCardHints({ ...v, ...change }, a), []);
    assert.deepEqual(researchAbilityHints({ ...v, ...change }, a), []);
  }
  for (const change of [{ action: null }, { reason: 'Already used' }, { researchAdvances: [] }]) {
    assert.deepEqual(researchCardHints({ ...v, actionCards: [{ ...source, ...change }] } as unknown as View, a), []);
    assert.deepEqual(researchAbilityHints({ ...v, specialActions: [{ ...source, ...change }] } as unknown as View, a), []);
  }
  assert.deepEqual(researchCardHints(v, { ...a, owned: true }), []);
  assert.deepEqual(researchAbilityHints(v, { ...a, owned: true }), []);
  assert.deepEqual(researchCardHints(v, { ...a, id: 'Metallurgy' }), []);
});

test('all ten research Great Persons expose exactly their actual free-advance choices', () => {
  for (const id of [118, 119, 120, 121, 122, 123, 124, 125, 126, 156]) {
    const g = fixture();
    g.players[0].advances = roots;
    g.players[0].action_cards = [id];
    const v = view(g), c = card(v, id);
    assert(c.action, `${id} should be playable`);
    assert(c.freeResearch);
    assert(c.researchAdvances!.length > 1, `${id} should offer a choice`);
    assert.deepEqual([...c.researchAdvances!].sort(), targets(view(move(g, c.action))), `${id} exact choices`);
    for (const id of c.researchAdvances!) assert(researchCardHints(v, advance(v, id)).includes(c));
  }
});

test('Scientist respects prerequisites, ownership, actions, pending decisions, and depleted groups', () => {
  const g = fixture();
  g.players[0].action_cards = [122];
  assert.deepEqual(card(view(g), 122).researchAdvances, ['Math']);
  g.players[0].advances.push('Math');
  let v = view(g);
  assert(researchCardHints(v, advance(v, 'Medicine')).length);
  g.players[0].resources = {};
  v = view(g);
  assert.equal(advance(v, 'Medicine').action, null);
  assert(researchCardHints(v, advance(v, 'Medicine')).length, 'free source remains useful without normal resources');
  g.actions_left = 0;
  assert.deepEqual(card(view(g), 122).researchAdvances, []);
  g.actions_left = 3;
  const pending = move(g, card(view(g), 122).action);
  assert.equal(view(pending).canPlay, false);
  assert.deepEqual(researchCardHints(view(pending), advance(view(pending), 'Medicine')), []);
  g.players[0].advances.push('Medicine', 'Astronomy');
  assert.deepEqual(card(view(g), 122).researchAdvances, ['Metallurgy'], 'singleton still advertised');
  g.players[0].advances.push('Metallurgy');
  const exhausted = card(view(g), 122);
  assert(exhausted.action, 'secondary effect is still playable');
  assert.deepEqual(exhausted.researchAdvances, []);
  assert.deepEqual(contextualCards(view(g), 'research'), []);
});

test('paid research and an explicitly chosen Scientist remain separate actions', () => {
  const g = fixture();
  g.players[0].advances.push('Math');
  g.players[0].action_cards = [122];
  const v = view(g), medicine = advance(v, 'Medicine');
  const paid = move(g, medicine.action);
  assert(paid.players[0].advances.includes('Medicine'));
  assert(paid.players[0].action_cards.includes(122));
  assert.equal(paid.actions_left, g.actions_left - 1);
  const requested = move(g, card(v, 122).action), decision = view(requested).decision!;
  assert.equal(decision.advanceMode, 'free');
  const selected = move(requested, advance(view(requested), 'Medicine').action);
  assert(selected.players[0].advances.includes('Medicine'));
  assert(!selected.players[0].action_cards.includes(122));
  assert.equal(selected.players[0].resources.food, g.players[0].resources.food);
  assert.equal(selected.players[0].resources.gold, g.players[0].resources.gold);
  assert.equal(selected.players[0].resources.ideas, g.players[0].resources.ideas + 1);
  assert.equal(selected.actions_left, g.actions_left - 1);
});

test('opponent and spectator views do not expose private research cards', () => {
  const g = fixture();
  g.players[0].action_cards = [122, 125, 124];
  g.players[1].action_cards = [];
  for (const seat of [1, null]) {
    const v = view(g, seat);
    assert.deepEqual(v.actionCards ?? [], []);
    assert.deepEqual(contextualCards(v, 'research'), []);
    for (const a of v.advances) assert.deepEqual(researchCardHints(v, a), []);
  }
  g.current_player_index = 1;
  const ownWaiting = view(g);
  assert(ownWaiting.actionCards!.every((c) => c.action === null && !c.researchAdvances?.length));
});

test('Shogunate free card allowance is respected and unavailable after once-per-turn use', () => {
  const g = fixture();
  g.players[0].civilization = 'Japan';
  g.players[0].advances = [...roots];
  g.players[0].action_cards = [122];
  g.actions_left = 0;
  const usable = card(view(g), 122);
  assert(usable.action);
  assert(usable.researchAdvances!.includes('Medicine'));
  g.players[0].event_info = { 'Shogunate card': 'used' };
  const used = card(view(g), 122);
  assert.equal(used.action, null);
  assert.deepEqual(used.researchAdvances, []);
});

const leaders = [
  ['Greece', 'Pericles', 'Master', 'Writing'],
  ['China', 'SunTzu', 'ArtOfWar', 'Tactics'],
  ['China', 'Wu', 'AgricultureEconomist', 'Farming'],
  ['Phoenicia', 'Hiram', 'Constructor', 'Mining'],
];
function leaderFixture(civilization: string, leader: string) {
  const g = fixture('civilizations/greece/master');
  g.players[0].civilization = civilization;
  g.players[0].units[0].unit_type = { Leader: leader };
  g.players[0].resources = { food: 6, mood_tokens: 3, culture_tokens: 3 };
  return g;
}
const ability = (v: View, kind: string) => v.specialActions?.find((a) => (a.action as any).Playing?.Custom?.action === kind);

test('four research leaders expose exact choices only with a happy leader city, token, and action', () => {
  for (const [civilization, leader, kind, root] of leaders) {
    const g = leaderFixture(civilization, leader);
    g.players[0].advances = [...new Set([...g.players[0].advances, root])];
    const v = view(g), offer = ability(v, kind)!;
    assert(offer, `${kind} available`);
    assert(offer.freeResearch);
    let requested = move(g, offer.action);
    // Resolve the ability's token payment, not a research resource payment.
    if (view(requested).decision?.fields.length) requested = move(requested, { Response: { Payment: [{ mood_tokens: 1 }] } });
    assert.deepEqual([...offer.researchAdvances!].sort(), targets(view(requested)), kind);
    assert(researchAbilityHints(v, advance(v, offer.researchAdvances![0])).includes(offer));
    g.players[0].cities[0].mood_state = 'Neutral';
    assert.equal(ability(view(g), kind), undefined, `${kind} needs happy city`);
    g.players[0].cities[0].mood_state = 'Happy';
    g.players[0].resources = {};
    assert.equal(ability(view(g), kind), undefined, `${kind} needs token`);
    g.players[0].resources = { mood_tokens: 3 };
    g.actions_left = 0;
    assert.equal(ability(view(g), kind), undefined, `${kind} needs action`);
  }
});

test('Golden Age reserves its culture fee and advertises only affordable unstarted groups', () => {
  const g = leaderFixture('India', 'SriGupta');
  g.actions_left = 0;
  const v = view(g), offer = ability(v, 'GoldenAge')!;
  assert(offer && offer.free && !offer.freeResearch);
  assert(offer.researchAdvances!.includes('Math'));
  assert(!offer.researchAdvances!.includes('Storage'));
  let requested = move(g, offer.action);
  if (view(requested).decision?.fields.length) requested = move(requested, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.deepEqual([...offer.researchAdvances!].sort(), targets(view(requested)));
  g.players[0].resources = { culture_tokens: 1 };
  assert.equal(ability(view(g), 'GoldenAge'), undefined);
  g.players[0].resources = { culture_tokens: 3, food: 6 };
  g.players[0].cities[0].angry_activation = true;
  assert.equal(ability(view(g), 'GoldenAge'), undefined);
});
