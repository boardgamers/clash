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
