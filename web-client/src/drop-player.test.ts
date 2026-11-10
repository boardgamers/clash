import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
async function bankingFixture() {
  const g = JSON.parse(await engine.init(4, [], {}, 'withdraw-bank', {}));
  assert.equal(g.players[2].civilization, 'Persia');
  assert.equal(g.players[3].civilization, 'Rome');
  for (const p of g.players) p.objective_cards = [];
  g.starting_player_index = 0;
  g.current_player_index = 1;
  g.age = 4;
  g.round = 2;
  g.players[2].advances.push('Currency');
  g.players[2].resources = { gold: 1 };
  g.permanent_effects = [{ AssassinationLoseAction: 3 }];
  const pending = JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify({ Playing: 'EndTurn' }), 1));
  assert.equal(pending.events[0].event_type, 'TurnStart');
  assert.deepEqual(pending.events[0].handler.origin, { SpecialAdvance: 'Banking' });
  return pending;
}

test('four-seat Banking withdrawal skips the reward and starts Rome exactly once', async () => {
  const g = await bankingFixture();
  const raw = await engine.dropPlayer(JSON.stringify(g), 2);
  const result = JSON.parse(raw);
  assert.equal(engine.currentPlayer(raw), 3);
  assert.equal(result.actions_left, 2);
  assert.equal(result.round, 2);
  assert.equal(result.events?.length ?? 0, 0);
  assert.deepEqual(result.players[2].resources, { gold: 1 });
  assert.equal(result.players.length, 6);
  assert.equal(await engine.dropPlayer(raw, 2), raw);
  const next = JSON.parse(engine.tryMove(raw, JSON.stringify({ Playing: 'EndTurn' }), 3));
  assert.equal(next.current_player_index, 0);
  assert.equal(next.round, 3);
});

test('legacy advanced-pointer Banking save normalizes consistently at every engine read', async () => {
  const g = await bankingFixture();
  g.dropped_players = [2];
  g.current_player_index = 3;
  const raw = JSON.stringify(g);
  assert.equal(engine.currentPlayer(raw), 3);
  assert.equal(engine.currentPlayer(raw), 3);
  const normalized = await engine.dropPlayer(raw, 2);
  assert.equal(JSON.parse(normalized).actions_left, 2);
  assert.equal(await engine.dropPlayer(normalized, 2), normalized);
  const move = JSON.stringify({ Playing: 'EndTurn' });
  assert.equal(engine.tryMove(raw, move, 3), engine.tryMove(normalized, move, 3));
});

test('opponent withdrawal does not cancel Persia Banking', async () => {
  const g = await bankingFixture();
  const result = JSON.parse(await engine.dropPlayer(JSON.stringify(g), 1));
  assert.deepEqual(result.events, g.events);
  assert.equal(engine.currentPlayer(JSON.stringify(result)), 2);
});

async function fixture() {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/combat/direct_capture_city_fortress.json', import.meta.url), 'utf8'));
  const initial = JSON.parse(await engine.init(2, [], {}, 'drop-player', {}));
  g.players.push(...initial.players.slice(2));
  return g;
}
for (const dropped of [0, 1]) test(`dropping seat ${dropped} ends the game with the remaining player winning regardless of points`, async () => {
  const g = await fixture();
  g.players[dropped].special_victory_points = [{ points: 100, origin: { Ability: "Test points" }, attribution: "Events" }];
  const scores = engine.scores(JSON.stringify(g));
  assert(scores[dropped] > scores[1 - dropped]);
  const raw = await engine.dropPlayer(JSON.stringify(g), dropped);
  const result = JSON.parse(raw);
  assert.equal(engine.ended(raw), true);
  assert.equal(result.current_player_index, 1 - dropped);
  assert.deepEqual(Array.from(engine.rankings(raw)), dropped === 0 ? [2, 1] : [1, 2]);
  assert.deepEqual(result.players.map((p: any) => p.units), g.players.map((p: any) => p.units));
  const repeated = JSON.parse(await engine.dropPlayer(raw, dropped));
  assert.deepEqual(repeated.dropped_players, [dropped]);
});
test('dropping during a pending battle clears decisions and ends the game', async () => {
  const g = await fixture();
  g.players[0].advances.push('Siegecraft');
  const pending = engine.tryMove(JSON.stringify(g), JSON.stringify({ Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1' } } }), 0);
  assert(JSON.parse(pending).events.length > 0);
  const raw = await engine.dropPlayer(pending, 0);
  assert.equal(engine.ended(raw), true);
  assert.equal(JSON.parse(raw).events?.length ?? 0, 0);
  assert.deepEqual(Array.from(engine.rankings(raw)), [2, 1]);
});
test('full replay repairs an old unfinished dropped game without replaying an extra move', async () => {
  const start = await engine.init(2, [], {}, 'drop-replay', {});
  const next = engine.tryMove(start, JSON.stringify({ Playing: 'EndTurn' }), engine.currentPlayer(start));
  const g = JSON.parse(next);
  g.dropped_players = [1];
  const saved = JSON.stringify(g);
  const result = engine.replay(saved, {});
  assert.equal(engine.ended(result), true);
  assert.deepEqual(Array.from(engine.rankings(result)), [1, 2]);
  assert.equal(engine.logLength(result), engine.logLength(saved));
  assert.equal(engine.ended(engine.replay(saved, { to: engine.logLength(start) })), false);
});
