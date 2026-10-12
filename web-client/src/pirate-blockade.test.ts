import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pirateBlockade } from './pirate-blockade.ts';
import { PirateBlockadeOverlay } from './pirate-blockade-overlay.ts';
import type { Game, Terrain } from './types.ts';

function board(): Pick<Game, 'map' | 'players'> {
  return {
    map: {
      tiles: ['A', 'B', 'C', 'D', 'E'].flatMap((col) =>
        [1, 2, 3, 4, 5].map((row): [string, Terrain] => [`${col}${row}`, 'Water']),
      ),
    },
    players: [
      {
        id: 0,
        civilization: 'Carthage',
        units: [{ id: 0, position: 'A1', unit_type: 'Ship', pirate: true }],
      },
      { id: 3, civilization: 'Pirates', units: [{ id: 0, position: 'C3', unit_type: 'Ship' }] },
    ],
  };
}

test('pirate blockade covers the occupied and adjacent sea spaces, never land or unexplored spaces', () => {
  const game = board();
  assert.deepEqual(pirateBlockade(game), ['B2', 'B3', 'C2', 'C3', 'C4', 'D2', 'D3']);
  game.map.tiles = game.map.tiles.map(([p, terrain]) => [
    p,
    p === 'C4' ? 'Fertile' : p === 'D2' ? 'Unexplored' : terrain,
  ]);
  assert.deepEqual(pirateBlockade(game), ['B2', 'B3', 'C2', 'C3', 'D3']);
  // Odd columns use the other pair of diagonal neighbors.
  game.players[1].units![0].position = 'B3';
  assert.deepEqual(pirateBlockade(game), ['A3', 'A4', 'B2', 'B3', 'B4', 'C3']);
});

test('blockade follows current or replayed pirate positions and ignores Carthage escorts', () => {
  const game = board();
  const previous = structuredClone(game);
  game.players[1].units = [];
  assert.deepEqual(pirateBlockade(game), [], 'allied pirate ships do not blockade');
  assert.ok(pirateBlockade(previous).includes('C3'), 'an older replay position still shows its pirates');
  game.players = game.players.slice(0, 1);
  assert.deepEqual(pirateBlockade(game), [], 'games without NPC pirates are supported');
});

test('blockade redraws only when positions change and disposes the removed outlines', () => {
  const overlay = new PirateBlockadeOverlay();
  overlay.update(pirateBlockade(board()));
  const old = overlay.group.children[0] as import('three').Mesh;
  let disposed = false;
  old.geometry.addEventListener('dispose', () => {
    disposed = true;
  });
  overlay.update(pirateBlockade(board()));
  assert.equal(overlay.group.children[0], old);
  overlay.update([]);
  assert.equal(overlay.group.children.length, 0);
  assert.equal(disposed, true);
  overlay.dispose();
});
