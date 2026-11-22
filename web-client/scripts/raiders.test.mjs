import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { raidersFixture } from './raiders-fixture.mjs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const guide = (game, seat = 1) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat)).tradeRoutes.find(p => p.player === 1);

test('Raiders metadata distinguishes two passengers from their trading ship, off-turn and public', async () => {
  const game = await raidersFixture(engine), before = JSON.stringify(game);
  const g = guide(game);
  assert.deepEqual(g.raidedSettlers, [{ position: 'D7', unit: 6 }, { position: 'D7', unit: 7 }]);
  assert.equal(g.routes.length, 1);
  assert.equal(g.routes[0].unit, 8);
  assert.deepEqual(guide(game, 0), g);
  assert.deepEqual(JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game)))).tradeRoutes.find(p => p.player === 1), g);
  assert.equal(JSON.stringify(game), before);
  game.players[0].cities[1].mood_state = 'Angry';
  assert.equal(guide(game).routes[0].to, 'D9');
  game.players[0].cities[0].mood_state = 'Angry';
  assert.equal(guide(game).routes.length, 0);
  assert.deepEqual(guide(game).raidedSettlers, g.raidedSettlers, 'angry destinations are not reported as extra raided sources');
});

test('own army aboard or outside, own city, and absent Raiders suppress the warning', async () => {
  const game = await raidersFixture(engine), player = game.players[1];
  player.units[0].carried_units.push({ unit_type: 'Infantry', id: 9 });
  assert.deepEqual(guide(game).raidedSettlers, []);
  player.units[0].carried_units = [];
  player.units.push({ position: 'C8', unit_type: 'Settler', id: 6 });
  assert.equal(guide(game).raidedSettlers.length, 1);
  player.units.push({ position: 'C8', unit_type: 'Infantry', id: 9 });
  assert.deepEqual(guide(game).raidedSettlers, []);
  player.units.pop();
  player.units.pop();
  // Disembark onto friendly land/city rather than creating a city on water.
  player.units.push({ position: 'D8', unit_type: 'Settler', id: 6 });
  assert.deepEqual(guide(game).raidedSettlers, []);
  player.units.pop();
  player.units[0].carried_units = [{ unit_type: 'Settler', id: 6 }];
  game.players[0].special_advances = [];
  game.players[0].advances = game.players[0].advances.filter(a => a !== 'Tactics');
  assert.deepEqual(guide(game).raidedSettlers, []);
});
