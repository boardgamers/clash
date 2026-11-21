import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { positionXY } from './model.ts';
import { strategyHome, strategyFrame } from './strategy-camera.ts';
import type { Game } from './types.ts';
const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');

test('new engine homes make one clockwise circuit in every viewer seat projection', async () => {
  for (const count of [2, 3, 4]) {
    const game: Game & { home_layout: string } = JSON.parse(
      await engine.init(count, [], {}, 'clockwise-viewer', {}),
    );
    assert.equal(game.home_layout, 'ClockwiseV1');
    const homes = game.players.slice(0, count).map((p) => strategyHome(game, p.id)!);
    if (count === 2) assert.deepEqual(homes, ['D2', 'D7']);
    for (let seat = 0; seat < count; seat++) {
      const frame = strategyFrame(game.map.tiles.map(([p]) => p), homes[seat], 1);
      const sin = Math.sin(frame.angle), cos = Math.cos(frame.angle);
      const points = homes.map((p) => {
        const [x, y] = positionXY(p);
        return [x * cos - y * sin, x * sin + y * cos];
      });
      const center = points.reduce(([x, y], p) => [x + p[0] / count, y + p[1] / count], [0, 0]);
      const angles = points.map(([x, y]) => Math.atan2(y - center[1], x - center[0]));
      const tau = 2 * Math.PI;
      const circuit = angles.reduce((sum, angle, i) => sum + ((angles[(i + 1) % count] - angle + tau) % tau), 0);
      assert.ok(Math.abs(circuit - tau) < 1e-9, `${count} players viewed from seat ${seat}`);
    }
  }
});
