import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { strategyHome, strategyFrame } from './strategy-camera.ts';
import { positionXY } from './model.ts';
import type { Game } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('every seat faces home-down on two-, three- and four-player boards and rotated bounds fit', async () => {
  for (const count of [2, 3, 4]) {
    const game: Game = JSON.parse(await engine.init(count, [], {}, 'home-orientation', {}));
    const positions = game.map.tiles.map(([p]) => p);
    for (let seat = 0; seat < count; seat++) {
      const home = strategyHome(game, seat)!;
      assert.equal(home, game.players[seat].cities![0].position);
      for (const aspect of [1400 / 800, 390 / 780]) {
        const frame = strategyFrame(positions, home, aspect);
        const [cx, cz] = frame.center;
        const sin = Math.sin(frame.angle),
          cos = Math.cos(frame.angle);
        const screen = (p: string) => {
          const [x, z] = positionXY(p);
          return [(x - cx) * cos - (z - cz) * sin, (x - cx) * sin + (z - cz) * cos];
        };
        const homeY = screen(home)[1];
        for (const opponent of game.players.slice(0, count).filter((p) => p.id !== seat))
          assert.ok(
            homeY > screen(opponent.cities![0].position)[1],
            `${count} players, seat ${seat} at bottom`,
          );
        const halfHeight = frame.distance * Math.tan(Math.PI / 10);
        for (const position of positions) {
          const [x, z] = screen(position);
          assert.ok(Math.abs(x) + 1 < halfHeight * aspect, 'rotated hex fits horizontally');
          assert.ok(Math.abs(z) + 1 < halfHeight, 'rotated hex fits vertically');
        }
      }
    }
    assert.equal(strategyHome(game), undefined, 'spectators have no home');
    assert.equal(strategyFrame(positions, undefined, 1).angle, 0, 'spectators retain normal orientation');
  }
});

test('original home survives conquest or relocation, with legacy setup and missing-history fallbacks', async () => {
  const game: Game = JSON.parse(await engine.init(2, [], {}, 'home-history', {}));
  const home = strategyHome(game, 0);
  game.players[0].cities = [];
  assert.equal(strategyHome(game, 0), home);
  game.log = [
    {
      age: 0,
      rounds: [
        {
          round: 0,
          turns: [
            { turn_type: 'Setup', actions: [{ log: ['Player1: Setup: Play as Greece, Gain city C2'] }] },
          ],
        },
      ],
    },
  ];
  assert.equal(strategyHome(game, 0), 'C2');
  game.log = [];
  game.players[0].cities = [{ position: 'E4', mood_state: 'Happy' }];
  assert.equal(strategyHome(game, 0), 'E4');
  assert.equal(strategyHome(game, 99), undefined);
});
