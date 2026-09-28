import { test } from 'node:test';
import assert from 'node:assert/strict';
import { seaArea, seaConnections } from './sea-routes.ts';

test('sea areas cross odd-column hex neighbors but stop at land and unexplored tiles', () => {
  const links = seaConnections([
    ['D2', 'Water'],
    ['E2', 'Water'],
    ['E3', 'Water'],
    ['D3', 'Unexplored'],
    ['F2', 'Barren'],
    ['G2', 'Water'],
  ]);
  assert.deepEqual([...seaArea('D2', links)].sort(), ['D2', 'E2', 'E3']);
  assert.deepEqual([...seaArea('G2', links)], ['G2']);
  assert.deepEqual([...seaArea(null, links)], []);
});
