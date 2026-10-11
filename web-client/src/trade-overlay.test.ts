import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TradeOverlay } from './trade-overlay.ts';

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
