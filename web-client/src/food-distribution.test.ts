import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (game: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), engine.currentPlayer(JSON.stringify(game))));

test('A good year allows a full-storage recipient and wastes their excess food', () => {
  let game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/good_year/good_year.json', import.meta.url), 'utf8'));
  const third = structuredClone(game.players[1]);
  third.id = 2;
  third.civilization = 'India';
  third.cities = [];
  third.units = [];
  third.resources.food = third.resource_limit.food;
  game.players[2].id = 3;
  game.players.splice(2, 0, third);
  const otherFood = game.players[1].resources.food;
  game = move(game, { Response: { SelectAdvance: 'Storage' } });
  game = move(game, { Response: { SelectUnitType: 'Elephant' } });
  const request = game.events.at(-1).handler.request.SelectPlayer;
  assert.deepEqual(request.choices, [1, 2]);
  game = move(game, { Response: { SelectPlayer: 2 } });
  assert.equal(game.players[2].resources.food, third.resource_limit.food);
  assert.equal(game.players[1].resources.food, otherFood);
});
