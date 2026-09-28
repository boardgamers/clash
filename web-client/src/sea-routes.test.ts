import { test } from 'node:test';
import assert from 'node:assert/strict';
import { seaArea, seaConnections } from './sea-routes.ts';
import { SeaOverlay } from './sea-overlay.ts';
import * as THREE from 'three';

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

test('sea previews work before enabling the guide, hide unrelated routes and clear on leave', () => {
  const overlay = new SeaOverlay();
  try {
    overlay.update(
      [
        ['D2', 'Water'],
        ['E2', 'Water'],
        ['G2', 'Water'],
      ],
      [],
    );
    const visibleRings = () =>
      overlay.group.children.filter(
        (object) =>
          object instanceof THREE.Mesh &&
          object.geometry instanceof THREE.TorusGeometry &&
          (object.material as THREE.MeshBasicMaterial).opacity > 0,
      ).length;
    assert.equal(overlay.group.visible, false);
    overlay.setFocus('D2');
    assert.equal(overlay.group.visible, true);
    assert.equal(visibleRings(), 2, 'Only the hovered sea and its connected neighbor are shown');
    overlay.setFocus('G2');
    assert.equal(visibleRings(), 1, 'Moving to an isolated sea clears the previous routes');
    overlay.setFocus(null);
    assert.equal(overlay.group.visible, false, 'Leaving the water hides the temporary preview');
    overlay.setFocus(null, true);
    assert.equal(overlay.group.visible, true);
    assert.equal(visibleRings(), 3, 'The guide can still show every sea together');
    overlay.setFocus(null, false);
    assert.equal(overlay.group.visible, false);
  } finally {
    overlay.dispose();
  }
});
