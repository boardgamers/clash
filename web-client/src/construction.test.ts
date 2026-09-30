import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'construction-tests', {})).players.slice(2);
function fixture(name: string) {
  const raw = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!raw.players.some((p: any) => p.civilization === npc.civilization))
      raw.players.push({ ...npc, id: raw.players.length });
  return raw;
}
const view = (game: any, seat = 0): View =>
  JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat));
const move = (game: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), 0));
const offer = (game: any, position = 'C2', name = 'Fortress') =>
  view(game)
    .cityActions.find((c) => c.position === position)!
    .buildings.find((b) => b.name === name)!;

test('Great Engineer exposes a paid-resource build with no extra action or city activation', () => {
  let game = fixture('incidents/great_persons/great_engineer');
  for (const action of [
    { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } },
    { Response: { Payment: [{ culture_tokens: 1 }] } },
  ])
    game = move(game, action);
  const actionsBeforeCard = game.actions_left;
  for (const action of [
    { Playing: { ActionCard: 126 } },
    { Response: { SelectAdvance: 'Engineering' } },
    { Response: { Bool: true } },
  ])
    game = move(game, action);
  const building = offer(game);
  assert.equal(building.source, 'Great Engineer');
  assert.equal(building.free, true);
  assert.equal(building.activateCity, false);
  assert.equal(building.moodWillDecrease, false);
  assert.equal(building.reason, null, 'Great Engineer permits building in a city that cannot activate again');
  assert.equal(
    Object.values(building.payment).reduce((sum, n) => sum + n, 0),
    3,
  );
  assert.ok(view(game, 1).cityActions.every((c) => c.buildings.every((b) => b.source === null && !b.free)));
  const cityBefore = game.players[0].cities.find((c: any) => c.position === 'C2');
  const resourcesBefore = { ...game.players[0].resources };
  game = move(game, building.choices[0].action);
  const cityAfter = game.players[0].cities.find((c: any) => c.position === 'C2');
  assert.equal(cityAfter.mood_state, cityBefore.mood_state);
  assert.equal(cityAfter.activations ?? 0, cityBefore.activations ?? 0);
  assert.equal(game.actions_left, actionsBeforeCard - 1, 'Only playing the card uses a turn action');
  for (const [resource, amount] of Object.entries(building.payment))
    assert.equal(game.players[0].resources[resource] ?? 0, (resourcesBefore[resource] ?? 0) - amount);
  assert.ok(
    view(game).cityActions.every((c) => c.buildings.every((b) => b.source === null && !b.free)),
    'The construction benefit ends after the build',
  );
});

test('City Development waives resources and the extra action while still activating the city', () => {
  let game = fixture('action_cards/city_development');
  game.players[0].cities.find((c: any) => c.position === 'C2').activations = 1;
  const actionsBeforeCard = game.actions_left;
  game = move(game, { Playing: { ActionCard: 17 } });
  game = move(game, { Response: { Payment: [{ culture_tokens: 1 }] } });
  const building = offer(game);
  assert.equal(building.source, 'City Development');
  assert.equal(building.free, true);
  assert.equal(building.activateCity, true);
  assert.equal(building.moodWillDecrease, true);
  assert.deepEqual(building.payment, {});
  assert.equal(building.reason, null);
  const resourcesBefore = { ...game.players[0].resources };
  game = move(game, building.choices[0].action);
  assert.deepEqual(game.players[0].resources ?? {}, resourcesBefore);
  assert.equal(game.players[0].cities.find((c: any) => c.position === 'C2').mood_state, 'Angry');
  assert.equal(game.actions_left, actionsBeforeCard - 1);
});

test('ordinary construction still spends one action, resources and a city activation', () => {
  let game = fixture('action_cards/city_development');
  game.players[0].resources = { food: 3, wood: 3, ore: 3 };
  game.players[0].cities.find((c: any) => c.position === 'C2').activations = 1;
  const building = offer(game);
  assert.equal(building.source, null);
  assert.equal(building.free, false);
  assert.equal(building.activateCity, true);
  assert.equal(building.moodWillDecrease, true);
  assert.deepEqual(building.payment, { food: 1, wood: 1, ore: 1 });
  const actionsBefore = game.actions_left;
  game = move(game, building.choices[0].action);
  assert.equal(game.actions_left, actionsBefore - 1);
  assert.deepEqual(game.players[0].resources, { food: 2, wood: 2, ore: 2 });
  assert.equal(game.players[0].cities.find((c: any) => c.position === 'C2').mood_state, 'Angry');
});
