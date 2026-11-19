import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { terrainDisplay } from './terrain-display.ts';
import { strategyTiles, strategyDescription } from './strategy.ts';
import type { Terrain } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');

for (const kind of ['Forest', 'Mountain', 'Fertile', 'Barren'] as const) {
  test(`exhausted ${kind} keeps its visual identity and separate exhaustion state`, () => {
    const terrain: Terrain = { Exhausted: kind };
    const normal = terrainDisplay(kind);
    assert.deepEqual(terrainDisplay(terrain), { ...normal, exhausted: true });
    assert.equal(normal.prominent, kind === 'Forest' || kind === 'Mountain');
    const [tile] = strategyTiles({ map: { tiles: [['A1', terrain]] }, players: [] });
    assert.deepEqual(tile.terrain, terrain);
    assert.equal(strategyDescription(tile), `Exhausted ${kind.toLowerCase()}`);
  });
}

test('exhausted terrain stays wrapped in the engine projection and cannot collect or found', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'exhausted-display', {}));
  const seat = game.current_player_index;
  const project = () => {
    const raw = engine.stripSecret(JSON.stringify(game), seat);
    return { raw, view: JSON.parse(engine.webView(raw, seat)) };
  };
  const city = project().view.cities[0];
  const choice = city.choices.find((c: any) => c.position !== city.position);
  assert.ok(choice, 'fixture has a collectable neighboring terrain');
  const tile = game.map.tiles.find(([p]: [string]) => p === choice.position);
  const original = tile[1];
  tile[1] = { Exhausted: original };
  const settler = game.players[seat].units.find((u: any) => u.unit_type === 'Settler');
  settler.position = tile[0];
  const { raw, view } = project();
  assert.deepEqual(JSON.parse(raw).map.tiles.find(([p]: [string]) => p === tile[0])[1], tile[1]);
  assert.ok(view.cities.every((c: any) => c.choices.every((option: any) => option.position !== tile[0])));
  const unit = view.settlers.find((u: any) => u.id === settler.id);
  assert.ok(unit);
  assert.equal(unit.foundAction, null);
  assert.throws(() =>
    engine.webCollectPreview(raw, seat, city.position, JSON.stringify([{ ...choice, times: 1 }])),
  );
});
