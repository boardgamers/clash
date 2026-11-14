import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { Engine, AchievementManifest, SeatAchievements } from '@boardgamers/protocol/engine';
import { achievementManifestSchema, achievementResultsSchema } from '@boardgamers/protocol/achievements';

const require = createRequire(import.meta.url);
// Check the generated WASM API, not a parallel JS achievement implementation.
const engine: typeof import('../.bridge/server.js') = require('../.engine/server.js');
const hooks = {
  achievementDefinitions: (): AchievementManifest => engine.achievementDefinitions(),
  achievements: (data: string): SeatAchievements[] => engine.achievements(data),
} satisfies Pick<Engine<string>, 'achievementDefinitions' | 'achievements'>;
const fixture = readFileSync(
  new URL('../../server/tests/test_games/status_phase/end_game.json', import.meta.url),
  'utf8',
);
const finish = () => engine.tryMove(fixture, JSON.stringify({ Playing: 'EndTurn' }), 0);

test('WASM achievement catalog follows official protocol 0.11.0', () => {
  const manifest = achievementManifestSchema.parse(hooks.achievementDefinitions());
  assert.equal(manifest.revision, 1);
  assert.equal(manifest.achievements.length, 17);
  assert.deepEqual(hooks.achievementDefinitions(), manifest);
  for (const length of ['shorter', 'standard', 'roll-to-end', 'wonderful-ending', 'epic']) {
    for (const verb of ['complete', 'win'])
      assert.ok(manifest.achievements.some((a) => a.id === `${verb}-${length}`));
  }
});

test('actual completed fixture produces valid, deterministic, immutable seat achievements', () => {
  assert.deepEqual(hooks.achievements(fixture), []);
  const finished = finish();
  assert.equal(engine.ended(finished), true);
  const before = JSON.parse(finished);
  const manifest = hooks.achievementDefinitions();
  const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(finished));
  assert.deepEqual(
    results.map((r) => r.seat),
    [0, 1],
  );
  const ranks = engine.rankings(finished) as number[];
  for (const result of results) {
    assert.ok(result.earned.includes('complete-standard'));
    const outcome =
      ranks[result.seat] === 1 ? (ranks.filter((r) => r === 1).length > 1 ? 'draw' : 'win') : 'loss';
    assert.equal(result.outcome, outcome);
    assert.equal(result.earned.includes('win-standard'), outcome === 'win');
  }
  assert.deepEqual(hooks.achievements(finished), results);
  assert.deepEqual(hooks.achievements(JSON.stringify(before)), results);
  assert.deepEqual(JSON.parse(finished), before);
});

test('WASM preserves inclusive score boundaries and awards feats to losing seats', () => {
  for (const [score, thirty, forty] of [
    [29.5, false, false],
    [30, true, false],
    [39.5, true, false],
    [40, true, true],
  ] as const) {
    const finished = finish();
    const data = JSON.parse(finished);
    const scores = engine.scores(finished) as number[];
    for (const seat of [0, 1]) {
      data.players[seat].special_victory_points ??= [];
      data.players[seat].special_victory_points.push({
        points: (seat === 0 ? score : 100) - scores[seat],
        origin: { Ability: 'achievement boundary test' },
        attribution: 'Events',
      });
    }
    const result = hooks.achievements(JSON.stringify(data)).find((r) => r.seat === 0)!;
    assert.equal(result.outcome, 'loss');
    assert.equal(result.earned.includes('score-30'), thirty);
    assert.equal(result.earned.includes('score-40'), forty);
    assert.ok(result.earned.includes('complete-standard'));
  }
});

test('all length options survive the serialized WASM boundary', () => {
  for (const [option, suffix] of [
    ['Shorter', 'shorter'],
    ['Standard', 'standard'],
    ['RollToEnd', 'roll-to-end'],
    ['WonderfulEnding', 'wonderful-ending'],
    ['Epic', 'epic'],
  ]) {
    const data = JSON.parse(finish());
    data.options = { ...data.options, length: option };
    const results = hooks.achievements(JSON.stringify(data));
    assert.equal(results.length, 2);
    for (const result of results) {
      assert.deepEqual(
        result.earned.filter((id) => id.startsWith('complete-')),
        [`complete-${suffix}`],
      );
    }
  }
});

test('filtered snapshots and all-withdrawn completions cannot award achievements', () => {
  const finished = finish();
  for (const seat of [undefined, 0, 1]) {
    assert.deepEqual(hooks.achievements(engine.stripSecret(finished, seat)), []);
  }
  const dropped = JSON.parse(finished);
  dropped.dropped_players = [0, 1];
  assert.deepEqual(hooks.achievements(JSON.stringify(dropped)), []);
});
