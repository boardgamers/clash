import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (suffix: string) =>
  readFileSync(
    new URL(`../../server/tests/test_games/incidents/pirates_spawn${suffix}.json`, import.meta.url),
    'utf8',
  );
const view = (raw: string, seat?: number): View =>
  JSON.parse(engine.webView(engine.stripSecret(raw, seat), seat));

test('pirate guide matches actual first and second placement requests for players and spectators', () => {
  for (const [suffix, phase] of [
    ['.outcome1', 'first'],
    ['.outcome2', 'second'],
  ] as const) {
    const raw = fixture(suffix);
    const state = JSON.parse(raw);
    const choices = state.events.at(-1).handler.request.SelectPositions.choices.sort();
    for (const seat of [undefined, 0, 1]) {
      assert.deepEqual(view(raw, seat).pirateSpawns?.find((p) => p.player === 0)?.[phase], choices);
    }
  }
});

test('pirate guide blocks human units, allows existing pirates, and falls back without coastal cities', () => {
  const state = JSON.parse(fixture('.outcome1'));
  const baseline = view(JSON.stringify(state)).pirateSpawns!.find((p) => p.player === 0)!;
  const blocked = baseline.second[0];
  state.players[0].units.push({ id: 999, position: blocked, unit_type: 'Ship' });
  state.players[2].units.push({ id: 998, position: baseline.second[1], unit_type: 'Ship' });
  state.players[0].cities = [];
  let guide = view(JSON.stringify(state)).pirateSpawns!.find((p) => p.player === 0)!;
  assert.deepEqual(guide.first, guide.second);
  assert.ok(!guide.second.includes(blocked));
  assert.ok(guide.second.includes(baseline.second[1]));
  state.map.tiles = state.map.tiles.map(([p, t]: [string, string]) => [
    p,
    p === baseline.second[1] ? 'Unexplored' : t,
  ]);
  guide = view(JSON.stringify(state)).pirateSpawns!.find((p) => p.player === 0)!;
  assert.ok(!guide.second.includes(baseline.second[1]));
});
