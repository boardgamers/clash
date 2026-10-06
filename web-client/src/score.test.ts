import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { formatPoints } from './score.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('victory points hide floating-point tails without losing Pyramid tenths', () => {
  for (const [raw, displayed] of [
    [41.099998474121094, '41.1'],
    [5.099999904632568, '5.1'],
    [41.599998474121094, '41.6'],
    [8.5, '8.5'],
    [0, '0'],
    [42, '42'],
  ] as const) {
    assert.equal(formatPoints(raw), displayed);
  }
});

test('building the Great Pyramid adds 5.1 displayed points in public scores', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'pyramid-score', {}));
  const scores = (state: unknown) =>
    JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(state), 0), 0)).players[0];
  const before = scores(game);
  game.players[0].wonders_built = ['Pyramids'];
  const after = scores(game);
  assert.equal(formatPoints(after.score - before.score), '5.1');
  assert.equal(formatPoints(after.score), formatPoints(before.score + 5.1));
  const wonderPoints = after.scoreParts.find(
    (part: { points: number }, index: number) => part.points !== before.scoreParts[index].points,
  );
  assert.equal(formatPoints(wonderPoints.points), '5.1');
});
