import assert from 'node:assert/strict';
import test from 'node:test';
import * as THREE from 'three';
import { thumbnailCamera } from './thumbnail-camera.ts';

test('thumbnail fits the entire board including tall pieces for wide and portrait captures', () => {
  for (const [width, height] of [
    [1200, 630],
    [400, 600],
  ]) {
    const bounds = new THREE.Box3(new THREE.Vector3(-8, 0, -5), new THREE.Vector3(14, 3, 24));
    const camera = thumbnailCamera(bounds, width, height);
    for (const x of [bounds.min.x, bounds.max.x])
      for (const y of [bounds.min.y, bounds.max.y])
        for (const z of [bounds.min.z, bounds.max.z]) {
          const projected = new THREE.Vector3(x, y, z).project(camera);
          assert.ok(
            Math.abs(projected.x) < 1 && Math.abs(projected.y) < 1,
            'board corner is inside the frame',
          );
          assert.ok(projected.z > -1 && projected.z < 1);
        }
  }
});
