import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
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
