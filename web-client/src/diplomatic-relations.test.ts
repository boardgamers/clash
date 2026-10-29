import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'diplomacy', {})).players.slice(2);
function fixture(owner = 0, target = 'army') {
  const game = JSON.parse(
    readFileSync(new URL('../../server/tests/test_games/movement/movement.json', import.meta.url), 'utf8'),
  );
  game.players.push(...structuredClone(npcs));
  game.players[0].advances.push('Tactics');
  game.players[owner].action_cards = [157];
  game.permanent_effects = [{ DiplomaticRelations: { active_player: owner, passive_player: 1 - owner } }];
  if (target !== 'army') game.players[1].units = [];
  if (target === 'settler') {
    game.players[1].cities[0].position = 'A3';
    game.players[1].units = [{ id: 0, position: 'C1', unit_type: 'Settler' }];
  }
  if (target === 'fortress') game.players[1].cities[0].city_pieces = { fortress: 1 };
  return game;
}
function destinations(game: unknown) {
  const raw = JSON.stringify(game);
  return JSON.parse(
    engine.webQuery(engine.stripSecret(raw, 0), 0, JSON.stringify({ kind: 'movement', units: [0] })),
  ).destinations;
}
const move = (game: unknown, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), 0));

for (const owner of [0, 1]) {
  for (const target of ['army', 'city', 'settler', 'fortress']) {
    test(`attacking a linked ${target} discards Great Diplomat held by player ${owner}`, () => {
      const game = fixture(owner, target);
      const offered = destinations(game);
      const attack = offered.find((d: any) => d.position === 'C1');
      assert.ok(attack?.attack);
      assert.equal(attack.breaksDiplomacy, true);
      assert.deepEqual(attack.payment, { culture_tokens: 2 });
      const after = move(game, attack.action);
      assert.equal(after.players[0].resources.culture_tokens, game.players[0].resources.culture_tokens - 2);
      assert.equal(after.players[1].resources.culture_tokens, game.players[1].resources.culture_tokens + 2);
      assert.ok(!after.permanent_effects?.some((e: any) => e.DiplomaticRelations));
      assert.ok(!after.players[owner].action_cards?.includes(157));
      assert.equal(after.action_cards_discarded.filter((id: number) => id === 157).length, 1);
      if (target === 'city') assert.ok(after.players[0].cities.some((c: any) => c.position === 'C1'));
    });
  }
}

test('an undefended linked city still requires the diplomatic payment', () => {
  const game = fixture(1, 'city');
  game.players[0].resources.culture_tokens = 1;
  assert.ok(!destinations(game).some((d: any) => d.position === 'C1'));
  assert.throws(() => move(game, { Movement: { Move: { units: [0], destination: 'C1' } } }));
});

test('attacking barbarians preserves diplomatic relations with another player and their card', () => {
  const game = fixture(1);
  const barbarians = game.players.find((p: any) => p.civilization === 'Barbarians');
  barbarians.units = [{ id: 0, position: 'B1', unit_type: 'Infantry' }];
  const attack = destinations(game).find((d: any) => d.position === 'B1');
  assert.ok(attack?.attack);
  assert.equal(attack.breaksDiplomacy, false);
  assert.deepEqual(attack.payment, {});
  const after = move(game, attack.action);
  assert.deepEqual(after.permanent_effects, game.permanent_effects);
  assert.ok(after.players[1].action_cards.includes(157));
  assert.ok(!after.action_cards_discarded?.includes(157));
});

for (const missingCard of [false, true]) test(`ending diplomacy is a regular action, immune to Subterfuge (legacy missing card ${missingCard})`, () => {
  const game = fixture();
  game.state = 'Playing';
  game.actions_left = 2;
  game.players[1].civilization = 'Japan';
  game.players[1].advances.push('Tactics');
  game.players[1].action_cards = [13];
  if (missingCard) {
    game.players[0].action_cards = [];
    game.action_cards_discarded = [157];
  }
  const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  assert.ok(!view.actionCards.some((c: any) => c.id === 157));
  assert.ok(view.specialActions.some((a: any) => a.name === 'End diplomatic relations' && !a.free));
  assert.throws(() => move(game, { Playing: { ActionCard: 157 } }));
  const after = move(game, { Playing: { Custom: { action: 'EndDiplomaticRelations' } } });
  assert.equal(after.actions_left, 1);
  assert.ok(!after.events?.length);
  assert.ok(!after.permanent_effects?.some((e: any) => e.DiplomaticRelations));
  assert.deepEqual(after.players[1].action_cards, [13]);
  assert.ok(!after.players[0].action_cards?.includes(157));
  assert.equal(after.action_cards_discarded.filter((id: number) => id === 157).length, 1);
  game.actions_left = 0;
  game.players[0].civilization = 'Japan';
  game.players[0].advances.push('Voting');
  assert.throws(() => move(game, { Playing: { Custom: { action: 'EndDiplomaticRelations' } } }));
});

test('only the treaty owner can end diplomatic relations', () => {
  const game = fixture(1);
  game.state = 'Playing';
  assert.throws(() => move(game, { Playing: { Custom: { action: 'EndDiplomaticRelations' } } }));
});

for (const warlord of [false, true]) test(`boarding another ship cannot move a land unit twice (Great Warlord ${warlord})`, () => {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/movement/ship_embark.json', import.meta.url), 'utf8'));
  game.players.push(...structuredClone(npcs));
  game.players[0].civilization = 'Japan';
  game.players[0].advances.push('Tactics');
  game.players[0].units = [
    { id: 0, unit_type: { Leader: 'GoToba' }, position: 'B3' },
    { id: 1, unit_type: 'Infantry', position: 'B3' },
    { id: 2, unit_type: 'Infantry', position: 'B3' },
    { id: 3, unit_type: 'Settler', position: 'B3' },
    { id: 7, unit_type: 'Ship', position: 'C3' },
    { id: 8, unit_type: 'Ship', position: 'C3' },
  ];
  game.state = { Movement: { movement_actions_left: 2, moved_units: [0], great_warlord_used: warlord } };
  const embark = (units: number[], carrier: number) => ({ Movement: { Move: { units, destination: 'C3', embark_carrier_id: carrier } } });
  const after = move(game, embark([1, 2], 7));
  assert.throws(() => move(after, embark([0], 8)));
  const offers = JSON.parse(engine.webQuery(engine.stripSecret(JSON.stringify(after), 0), 0, JSON.stringify({kind:'movement',units:[0]}))).destinations;
  assert.equal(offers.length, 0);
  const valid = move(after, embark([3], 8));
  assert.equal(valid.state.Movement.movement_actions_left, 1, 'fresh passengers share the same group move');
});
