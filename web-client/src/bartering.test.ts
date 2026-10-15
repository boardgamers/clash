import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'barter-npcs', {})).players.slice(2);
function fixture() {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/advances/bartering.json', import.meta.url), 'utf8'));
  g.players.push(...npcs);
  Object.assign(g.players[0], { action_cards: [1], objective_cards: [1], wonder_cards: ['GreatGardens'] });
  g.players[0].resources.gold = 0;
  return g;
}
const view = (g: any) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const move = (g: any, action: any) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
for (const [field, card] of [
  ['action_cards', { ActionCard: 1 }], ['objective_cards', { ObjectiveCard: 1 }], ['wonder_cards', { Wonder: 'GreatGardens' }],
] as const) {
  test(`Bartering offers and discards ${field}, grants one reward and remains once per turn`, () => {
    const g = fixture();
    const offer = view(g).specialActions.find((a: any) => a.name === 'Bartering');
    assert.ok(offer);
    const choosing = move(g, offer.action);
    assert.match(JSON.stringify(view(choosing).decision), /GreatGardens/);
    const discarded = move(choosing, { Response: { SelectHandCards: [card] } });
    assert.equal((discarded.players[0][field] ?? []).length, 0);
    for (const other of ['action_cards', 'objective_cards', 'wonder_cards'])
      if (other !== field) assert.deepEqual(discarded.players[0][other], g.players[0][other]);
    const rewarded = move(discarded, { Response: { ResourceReward: { gold: 1 } } });
    assert.equal(rewarded.players[0].resources.gold, 1);
    assert.equal(rewarded.actions_left, g.actions_left);
    assert.ok(!view(rewarded).specialActions.some((a: any) => a.name === 'Bartering' && a.action));
  });
  test(`Bartering remains available with only ${field} in hand`, () => {
    const g = fixture();
    for (const other of ['action_cards', 'objective_cards', 'wonder_cards']) if (other !== field) g.players[0][other] = [];
    assert.ok(view(g).specialActions.some((a: any) => a.name === 'Bartering' && a.action));
  });
}
