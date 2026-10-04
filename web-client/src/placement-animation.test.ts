import { test, type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { PlacementAnimation, PLACEMENT_DURATION } from './placement-animation.ts';

function animationClock(t: TestContext) {
  let now = 0;
  let serial = 0;
  const pending = new Map<number, FrameRequestCallback>();
  const request = globalThis.requestAnimationFrame;
  const cancel = globalThis.cancelAnimationFrame;
  globalThis.requestAnimationFrame = (callback) => {
    pending.set(++serial, callback);
    return serial;
  };
  globalThis.cancelAnimationFrame = (id) => {
    pending.delete(id);
  };
  t.mock.method(performance, 'now', () => now);
  t.after(() => {
    globalThis.requestAnimationFrame = request;
    globalThis.cancelAnimationFrame = cancel;
  });
  return {
    advance(ms: number) {
      now += ms;
      const callbacks = [...pending.values()];
      pending.clear();
      for (const callback of callbacks) callback(now);
    },
    pending,
  };
}

test('new buildings rise and units arrive at their actual world positions, then settle exactly', (t) => {
  const clock = animationClock(t);
  const animation = new PlacementAnimation();
  const city = new THREE.Group();
  city.position.set(5, 0.5, 3);
  city.rotation.y = Math.PI / 3;
  const building = new THREE.Group();
  building.position.set(0.2, 0, -0.3);
  building.scale.setScalar(0.62);
  city.add(building);
  const unit = new THREE.Group();
  unit.position.set(2, 0.43, 3);
  unit.scale.setScalar(0.75);
  const unchanged = new THREE.Group();
  city.add(unchanged);
  const position = building.position.clone(),
    scale = building.scale.clone();
  const unitPosition = unit.position.clone(),
    unitScale = unit.scale.clone();
  const at = building.getWorldPosition(new THREE.Vector3());
  let renders = 0;
  animation.play(
    [
      { model: building, kind: 'building' },
      { model: unit, kind: 'unit' },
    ],
    () => renders++,
  );
  assert.ok(building.scale.y < scale.y / 10);
  assert.ok(unit.position.y > unitPosition.y);
  assert.equal(animation.group.children.length, 2);
  assert.equal(animation.group.children[0].position.x, at.x);
  assert.equal(animation.group.children[0].position.z, at.z);
  let disposed = 0;
  for (const child of animation.group.children as THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>[]) {
    child.geometry.addEventListener('dispose', () => disposed++);
    child.material.addEventListener('dispose', () => disposed++);
  }
  clock.advance(350);
  assert.ok(building.scale.y > scale.y / 2 && building.scale.y < scale.y);
  assert.ok(unit.position.y > unitPosition.y && unit.position.y < unitPosition.y + 0.4);
  assert.deepEqual(unchanged.scale.toArray(), [1, 1, 1]);
  clock.advance(PLACEMENT_DURATION);
  assert.deepEqual(building.position, position);
  assert.deepEqual(building.scale, scale);
  assert.deepEqual(unit.position, unitPosition);
  assert.deepEqual(unit.scale, unitScale);
  assert.equal(animation.group.children.length, 0);
  assert.equal(disposed, 4);
  assert.equal(clock.pending.size, 0);
  assert.ok(renders >= 3);
});

test('interrupting an appearance restores models and removes all animation resources', (t) => {
  const clock = animationClock(t),
    animation = new PlacementAnimation(),
    model = new THREE.Group();
  model.scale.set(0.4, 0.7, 0.5);
  const scale = model.scale.clone();
  animation.play([{ model, kind: 'building' }], () => {});
  clock.advance(100);
  animation.stop();
  animation.stop();
  assert.deepEqual(model.scale, scale);
  assert.equal(clock.pending.size, 0);
  assert.equal(animation.group.children.length, 0);
  animation.play([{ model, kind: 'unit' }], () => {});
  animation.play([], () => {});
  assert.deepEqual(model.scale, scale);
  assert.equal(clock.pending.size, 0);
  assert.equal(animation.group.children.length, 0);
});
