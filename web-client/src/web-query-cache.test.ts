import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = JSON.parse(readFileSync(new URL('../../server/tests/test_games/advances/sanitation_and_draft.json', import.meta.url), 'utf8'));
const npcs = JSON.parse(await engine.init(2, [], {}, 'query-cache', {})).players.slice(2);
for (const npc of npcs) if (!fixture.players.some((p: any) => p.civilization === npc.civilization))
  fixture.players.push({ ...npc, id: fixture.players.length });
const raw = JSON.stringify(fixture);
const view = (state: string, seat?: number) => {
  const result = JSON.parse(engine.webView(state, seat));
  // Player advances originate in a Rust HashMap; ordering is not part of the view contract.
  for (const p of result.players) p.advances.sort((a: any, b: any) => a.id.localeCompare(b.id));
  return result;
};
const query = (state: string, seat: number, input: unknown) => JSON.parse(engine.webQuery(state, seat, JSON.stringify(input)));

test('read-only game cache respects exact state, viewer seat and returning to an older state', () => {
  const original = view(raw, 0);
  const spectator = view(raw);
  assert.deepEqual(view(raw, 0), original);
  const changed = JSON.parse(raw);
  changed.players[0].resources.food = 0;
  const updated = view(JSON.stringify(changed), 0);
  assert.notDeepEqual(updated, original);
  assert.deepEqual(view(raw, 0), original);
  assert.deepEqual(view(raw), spectator);
  assert.deepEqual(view(raw + ' ', 0), original, 'freshly parsed game matches cached result');
});

test('queries and local moves cannot mutate the cached view', () => {
  const before = view(raw, 0);
  const input = { kind: 'researchPlan', action: { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } } };
  const first = query(raw, 0, input);
  assert.deepEqual(query(raw, 0, input), first);
  assert.deepEqual(query(raw + ' ', 0, input), first);
  const moved = engine.tryMove(raw, JSON.stringify({ Playing: 'EndTurn' }), 0);
  assert.notEqual(moved, raw);
  assert.deepEqual(view(raw, 0), before);
});
