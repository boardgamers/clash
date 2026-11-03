import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (game: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), engine.currentPlayer(JSON.stringify(game))));

function fixture() {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/good_year/good_year.json', import.meta.url), 'utf8'));
  const third = structuredClone(game.players[1]);
  third.id = 2;
  third.civilization = 'India';
  third.cities = [];
  third.units = [];
  third.resources.food = third.resource_limit.food;
  game.players[2].id = 3;
  game.players.splice(2, 0, third);
  return game;
}

test('A good year must give food to the opponent with room instead of wasting it on a full recipient', () => {
  let game = fixture();
  const otherFood = game.players[1].resources.food;
  game = move(game, { Response: { SelectAdvance: 'Storage' } });
  game = move(game, { Response: { SelectUnitType: 'Elephant' } });
  assert.ok(!game.events.some((event: any) => event.handler?.request?.SelectPlayer));
  assert.equal(game.players[1].resources.food, otherFood + 1, 'the sole eligible recipient is automatic');
  assert.equal(game.players[2].resources.food, game.players[2].resource_limit.food);
});

test('A good year still permits unavoidable waste when every other player is full', () => {
  let game = fixture();
  game.players[1].resources.food = game.players[1].resource_limit.food;
  game = move(game, { Response: { SelectAdvance: 'Storage' } });
  game = move(game, { Response: { SelectUnitType: 'Elephant' } });
  assert.deepEqual(game.events.at(-1).handler.request.SelectPlayer.choices, [1, 2]);
  game = move(game, { Response: { SelectPlayer: 2 } });
  for (const index of [1, 2]) assert.equal(game.players[index].resources.food, game.players[index].resource_limit.food);
});
