import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
const apply = (raw: string, action: unknown, player = engine.currentPlayer(raw)) => engine.tryMove(raw, JSON.stringify(action), player);
const view = (raw: string, player: number) => JSON.parse(engine.webView(engine.stripSecret(raw, player), player));
const city = { id: 0, player_id: 0, mood_state: 'Happy', position: 'C1', pieces: { wonders: [] } };
const infantry = (id: number, position: string) => ({ id, player_id: 1, position, unit_type: 'Infantry' });
function retreat(defenders: number) {
  const g = fixture('tactics_cards/heavy_resistance');
  g.players[0].action_cards = [];
  g.players[0].advances.push('Engineering');
  g.players[0].cities = [];
  g.players[1].civilization = 'Persia';
  g.players[1].action_cards = [19];
  g.players[1].units = Array.from({ length: defenders }, (_, i) => infantry(i, 'C1'));
  g.players[1].units.push({ id: 10, player_id: 1, position: 'C1', unit_type: 'Settler' });
  g.players[1].units.push(infantry(11, 'B1'));
  g.players[1].next_unit_id = 12;
  return g;
}
test('actual Tactical Retreat options include stack limits and the full group', () => {
  let raw = apply(JSON.stringify(retreat(1)), { Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }, 0);
  assert.equal(view(raw, 1).decision.options.length, 1);
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 19 }] } }, 1);
  const g = JSON.parse(raw);
  assert.equal(g.players[1].units.filter((u: any) => u.position === 'B1').length, 3);
  assert.equal(g.players[1].units.filter((u: any) => u.position === 'C1').length, 0);
  assert(g.players[1].units.some((u: any) => u.unit_type === 'Settler' && u.position === 'B1'));
  assert.equal(g.players[0].units.length, 8);
});
test('actual Tactical Retreat remains playable when every destination would overflow', () => {
  const g = retreat(2);
  g.players[1].units.push(infantry(12, 'B2'), infantry(13, 'B2'), infantry(14, 'B2'));
  let raw = apply(JSON.stringify(g), { Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }, 0);
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 19 }] } }, 1);
  assert(JSON.stringify(JSON.parse(raw).log).includes('withdraws to'));
});
async function destroyedWonder(wonder: string, civilization: string) {
  const g = fixture('civilizations/china/great_wall');
  const npcs = JSON.parse(await engine.init(2, [], {}, 'wonder-regression', {})).players.slice(2);
  g.players.push(...npcs);
  const leader = civilization === 'China' ? 'Qin' : 'Hammurabi';
  g.players[0].civilization = civilization;
  g.players[0].units[0].unit_type = { Leader: leader };
  g.players[0].recruited_leaders = [leader];
  g.players[0].wonder_cards = [wonder];
  g.players[0].resources = { food: 9, wood: 9, ore: 9, ideas: 9, gold: 9, mood_tokens: 9, culture_tokens: 9 };
  g.players[0].cities.push(city);
  for (const advance of ['Engineering', 'Irrigation', 'Math', 'Astronomy']) if (!g.players[0].advances.includes(advance)) g.players[0].advances.push(advance);
  g.wonders_left = g.wonders_left.filter((w: string) => w !== wonder);
  g.players[1].wonder_cards = [];
  let raw = apply(JSON.stringify(g), { Playing: { WonderCard: wonder } }, 0);
  raw = apply(raw, { Response: { SelectPositions: ['D1'] } }, 0);
  const payment = wonder === 'GreatWall' ? { food: 3, wood: 2, ore: 7, culture_tokens: 3 } : { food: 5, wood: 5, ore: 2, culture_tokens: 5 };
  raw = apply(raw, { Response: { Payment: [payment] } }, 0);
  let state = JSON.parse(raw);
  assert.equal(state.events?.length ?? 0, 0, JSON.stringify(state.events));
  assert(state.players[0].cities.find((c: any) => c.position === 'D1').city_pieces.wonders.includes(wonder));
  state.players[0].special_advances = [];
  state.players[0].incident_tokens = 1;
  state.incidents_left = [30];
  state.wonders_left = ['Pyramids'];
  state.permanent_effects = [];
  state.actions_left = 1;
  raw = apply(JSON.stringify(state), { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } }, 0);
  if (civilization === 'Babylonia') {
    // Astronomy enables Star Catalogues, which legitimately resolves before the incident.
    raw = apply(raw, { Response: { ResourceReward: { ideas: 1 } } }, 0);
  }
  raw = apply(raw, { Response: { SelectStructures: [{ position: 'D1', structure: { Wonder: wonder } }] } }, 0);
  state = JSON.parse(raw);
  assert.equal(state.events?.length ?? 0, 0, JSON.stringify(state.events));
  assert.deepEqual(state.destroyed_wonders, [wonder]);
  state.players[0].advances = state.players[0].advances.filter((a: string) => !['Roads', 'Engineering'].includes(a));
  state.actions_left = 1;
  raw = apply(JSON.stringify(state), { Playing: { Advance: { advance: 'Engineering', payment: {} } } }, 0);
  state = JSON.parse(raw);
  assert.deepEqual(state.players[0].wonder_cards, ['Pyramids']);
  assert.equal(state.players[0].wonders_built?.length ?? 0, 0);
  for (const seat of [0, 1, null]) {
    const publicState = JSON.parse(engine.stripSecret(raw, seat));
    assert.deepEqual(publicState.destroyed_wonders, [wonder]);
  }
  assert.deepEqual(JSON.parse(raw).destroyed_wonders, [wonder]);
  const original = fixture('civilizations/china/great_wall');
  assert.equal(original.destroyed_wonders, undefined);
}
test('Qin and Babylonia use ordinary Engineering draws after real Earthquake destruction', async () => {
  await destroyedWonder('GreatWall', 'China');
  await destroyedWonder('GreatGardens', 'Babylonia');
});
test('Defiant Stand journal reports one total arithmetic penalty', () => {
  let raw = JSON.stringify(fixture('tactics_cards/heavy_resistance'));
  raw = apply(raw, { Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }, 0);
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 7 }] } }, 1);
  assert(JSON.stringify(JSON.parse(raw).log).includes('Defiant Stand: -1 total combat value (1 per attacking unit)'));
});
