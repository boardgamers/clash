import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MapGesture } from './map-gesture.ts';

test('map clicks tolerate small pointer movement but a drag returning to its origin never selects', () => {
  const gesture = new MapGesture();
  gesture.down(1, 100, 100);
  assert.equal(gesture.up(1, 102, 102), true);
  gesture.down(1, 100, 100);
  gesture.move(130, 120);
  assert.equal(gesture.dragging, true);
  gesture.move(100, 100);
  assert.equal(gesture.up(1, 100, 100), false);
  assert.equal(gesture.dragging, false);
  assert.equal(gesture.up(9, 100, 100), false, 'Releasing a pointer that started outside is not a click');
});

test('pinching and cancelled touch gestures never select a tile, while the next tap still works', () => {
  const gesture = new MapGesture();
  gesture.down(1, 20, 20);
  gesture.down(2, 40, 40);
  assert.equal(gesture.up(2, 40, 40), false);
  assert.equal(gesture.up(1, 20, 20), false);
  gesture.down(1, 20, 20);
  gesture.cancel(1);
  assert.equal(gesture.up(1, 20, 20), false);
  gesture.down(1, 20, 20);
  assert.equal(gesture.up(1, 20, 20), true);
});
