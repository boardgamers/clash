import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TradeOverlay } from './trade-overlay.ts';
import * as THREE from 'three';
import { tileSurfaceHeight } from './terrain-display.ts';
import type { Terrain } from './types.ts';

test('trade overlay draws pairings and replaces or clears old geometry', () => {
  const overlay = new TradeOverlay();
  const routes = [{ from: 'C2', to: 'A1' }, { from: 'B2', to: 'D2' }];
  overlay.update(routes, '#f4c86b');
  assert.equal(overlay.group.children.length, 6);
  const first = overlay.group.children[0];
  overlay.update(routes, '#f4c86b');
  assert.equal(overlay.group.children[0], first);
  overlay.update([routes[0]], '#f4c86b');
  assert.equal(overlay.group.children.length, 3);
  overlay.update([], '#f4c86b');
  assert.equal(overlay.group.children.length, 0);
  overlay.dispose();
});

test('Raiders source contours deduplicate a shared tile without hiding its valid ship route', () => {
  const overlay = new TradeOverlay();
  overlay.update([{ from: 'D7', to: 'D9' }], '#f4c86b', ['D7', 'D7']);
  assert.equal(overlay.group.children.length, 3, 'valid route, destination endpoint, and one solid orange source contour');
  assert.equal((overlay.group.children[0] as any).geometry.type, 'TubeGeometry', 'ship route line remains visible');
  assert.equal((overlay.group.children[2] as any).material.color.getHexString(), 'ed8a23');
  overlay.update([{ from: 'D7', to: 'D9' }, { from: 'C2', to: 'A1' }], '#f4c86b', ['D7']);
  assert.equal(overlay.group.children.length, 6, 'both valid routes remain visible');
  overlay.update([{ from: 'D7', to: 'D9' }], '#f4c86b', []);
  assert.equal(overlay.group.children.length, 3, 'protection restores the normal route');
  overlay.update([], '#f4c86b');
  assert.equal(overlay.group.children.length, 0);
});

for (const terrain of ['Water', 'Fertile', 'Mountain', 'Unexplored', { Exhausted: 'Mountain' }] satisfies Terrain[]) {
  test(`Raiders contour hugs the slab below pieces: ${JSON.stringify(terrain)}`, () => {
    const overlay = new TradeOverlay();
    overlay.update([{ from: 'D7', to: 'D9' }], '#f4c86b', ['D7'], [['D7', terrain], ['D9', terrain]]);
    const ring = overlay.group.children[2] as THREE.Mesh<THREE.TorusGeometry, THREE.MeshBasicMaterial>;
    const surface = tileSurfaceHeight(terrain);
    assert(Math.abs(ring.position.y - surface - 0.04) < 1e-8);
    assert(ring.position.y - ring.geometry.parameters.tube > surface, 'tube clears slab without z-fighting');
    assert(ring.position.y + ring.geometry.parameters.tube < surface + 0.07, 'contour remains at ground level, not above unit bodies');
    assert(ring.geometry.parameters.radius + ring.geometry.parameters.tube < 0.976, 'contour stays inside tile perimeter');
    assert.equal(ring.material.depthTest, true, 'terrain and unit silhouettes occlude the contour');
    assert.equal(ring.material.depthWrite, false);
    assert.equal(overlay.group.children[1].position.y, ring.position.y, 'gold and orange endpoints share slab-relative height');
    assert.equal((overlay.group.children[0] as THREE.Mesh).geometry.type, 'TubeGeometry', 'valid ship route survives');
    overlay.update([], '#f4c86b', ['D7'], [['D7', 'Water']]);
    assert(Math.abs(overlay.group.children[0].position.y - 0.05) < 1e-8, 'terrain changes invalidate cached elevation');
    overlay.dispose();
  });
}
