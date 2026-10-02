import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { explorationPreview, unexploredRegion } from './exploration-preview.ts';
import type { Game, Session } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

for (const players of [2, 3, 4]) {
  test(`${players}-player public map outlines cover exactly its unrevealed regions`, async () => {
    const raw = await engine.init(players, [], { civilization: 'Random' }, 'exploration-preview', {});
    const seat = engine.currentPlayer(raw);
    const game: Game = JSON.parse(engine.stripSecret(raw, seat));
    const unexplored = game.map.tiles.filter(([, t]) => t === 'Unexplored').map(([p]) => p);
    const regions = new Set<string>();
    for (const position of unexplored) {
      const region = unexploredRegion(game.map, position);
      assert.equal(new Set(region).size, 4);
      assert(
        region.every((p) => unexplored.includes(p)),
        'only unrevealed hexes outlined',
      );
      regions.add(region.sort().join(' '));
    }
    assert.equal(regions.size, game.map.unexplored_blocks!.length);
    assert.equal([...regions].flatMap((r) => r.split(' ')).length, unexplored.length);

    const view = JSON.parse(engine.webView(JSON.stringify(game), seat));
    for (const destination of view.settlers[0].destinations.filter(
      (d: { terrain: string }) => d.terrain === 'Unexplored',
    )) {
      const expected = unexploredRegion(game.map, destination.position).sort();
      const after = engine.tryMove(raw, JSON.stringify(destination.action), seat);
      const revealed: Game = JSON.parse(engine.stripSecret(after, seat));
      const nextView = JSON.parse(engine.webView(JSON.stringify(revealed), seat));
      const actual =
        nextView.explorationDecision?.choices[0]?.tiles.map(([p]: [string, unknown]) => p) ??
        revealed.map.tiles.filter(([p, t]) => t !== 'Unexplored' && unexplored.includes(p)).map(([p]) => p);
      assert.deepEqual(actual.sort(), expected, 'preview matches actual engine reveal');
    }
    assert.deepEqual(
      unexploredRegion({ tiles: game.map.tiles }, unexplored[0]),
      [],
      'older maps without block placement stay safe',
    );
  });
}

test('only the current legal unexplored destination gets a preview', async () => {
  const raw = await engine.init(2, [], { civilization: 'Random' }, 'exploration-selection', {});
  const seat = engine.currentPlayer(raw);
  const game: Game = JSON.parse(engine.stripSecret(raw, seat));
  const view = JSON.parse(engine.webView(JSON.stringify(game), seat));
  const destinations = view.settlers[0].destinations;
  const target = destinations.find((d: { terrain: string }) => d.terrain === 'Unexplored').position;
  const s: Parameters<typeof explorationPreview>[0] = {
    game,
    mode: 'settlers',
    moveTarget: target,
    moveDestinations: destinations,
    playback: null,
  };
  assert.equal(explorationPreview(s).length, 4);
  assert.deepEqual(explorationPreview({ ...s, moveTarget: null }), []);
  assert.deepEqual(explorationPreview({ ...s, mode: 'overview' }), []);
  assert.deepEqual(explorationPreview({ ...s, moveDestinations: [] }), []);
  assert.deepEqual(explorationPreview({ ...s, playback: {} as NonNullable<Session['playback']> }), []);
  assert.deepEqual(explorationPreview({ ...s, moveTarget: view.settlers[0].position }), []);
});
