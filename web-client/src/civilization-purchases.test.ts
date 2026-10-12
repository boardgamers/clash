import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { paymentWithAmount } from './payment-options.ts';
import { aqueductResearch, researchCardHints } from './research-links.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'purchases', {})).players.slice(2);
function fixture(name: string) {
  const g = JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
  for (const npc of npcs) if (!g.players.some((p: any) => p.civilization === npc.civilization)) g.players.push({ ...npc, id: g.players.length });
  return g;
}
const view = (g: any) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const move = (g: any, a: any) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(a), 0));

test('Carthage ship recruitment exposes exact discounted gold payment without extra wood', () => {
  const g = fixture('advances/sanitation_and_draft'), p = g.players[0];
  p.civilization = 'Carthage'; p.resources = { gold: 7, wood: 7, mood_tokens: 7 };
  p.advances = [...new Set([...p.advances, 'Fishing'])];
  const city = p.cities.find((c: any) => c.position === 'C2');
  city.mood_state = 'Happy'; city.activations = 0; city.angry_activation = false;
  const query = (payment?: object) => JSON.parse(engine.webQuery(engine.stripSecret(JSON.stringify(g), 0), 0,
    JSON.stringify({ kind: 'recruit', units: { ships: 1 }, city: 'C2', replaced: [], payment })));
  const preview = query();
  assert.deepEqual(paymentWithAmount(preview.payments, { wood: 2 }, 'gold', 1), { gold: 1 });
  const paid = move(g, query({ gold: 1 }).action);
  assert.equal(paid.players[0].resources.gold, 6);
  assert.equal(paid.players[0].resources.wood, 7);
  p.advances.push('Voting', 'CivilLiberties');
  assert(!query().payments.some((p: Record<string, number>) => p.gold === 1 && Object.keys(p).length === 1));
  assert.equal(move(g, query({ gold: 2 }).action).players[0].resources.gold, 5);
});

test('Sanitation exposes Aqueduct free action after main actions run out and preserves payment choice', () => {
  const g = fixture('civilizations/rome/aqueduct_free_action');
  g.actions_left = 0;
  const v = view(g), sanitation = v.advances.find((a: any) => a.id === 'Sanitation');
  assert.equal(sanitation.action, null);
  const alternative = aqueductResearch(v, sanitation)!;
  assert(alternative && !alternative.reason && alternative.free);
  const prompted = move(g, alternative.action);
  assert.equal(view(prompted).decision.fields.length, 1);
  const paid = move(prompted, { Response: { Payment: [{ food: 2 }] } });
  assert(paid.players[0].advances.includes('Sanitation'));
  assert.equal(paid.actions_left ?? 0, 0);
  assert.equal((paid.players[0].resources.food ?? 0), (g.players[0].resources.food ?? 0) - 2);
  assert.equal(aqueductResearch(view(paid), view(paid).advances.find((a: any) => a.id === 'Sanitation')), undefined);
});


test('Synergies research hints match the actual first-advance choices and disappear when unavailable', () => {
  const g = fixture('action_cards/synergies');
  g.players[0].action_cards = [33];
  const v = view(g), card = v.actionCards.find((c: any) => c.id === 33);
  assert(card?.action);
  assert(card.researchAdvances.length > 0);
  const prompted = view(move(g, card.action));
  assert(prompted.decision);
  assert.deepEqual([...card.researchAdvances].sort(), prompted.decision.options.map((o: any) => o.value).sort());
  for (const id of card.researchAdvances) {
    const advance = v.advances.find((a: any) => a.id === id);
    assert.equal(advance.owned, false);
    assert(researchCardHints(v, advance).some((c: any) => c.id === 33));
  }
  g.players[0].resources = {};
  assert.deepEqual(view(g).actionCards.find((c: any) => c.id === 33).researchAdvances, []);
});
