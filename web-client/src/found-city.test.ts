import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('a settler can found a forest city after movement, but needs a separate action', async () => {
  let raw = await engine.init(2, [], { civilization: 'Random' }, 'founding-in-forest', {});
  const seat = engine.currentPlayer(raw);
  const view = () => JSON.parse(engine.webView(engine.stripSecret(raw, seat), seat));
  const move = view().settlers[0].destinations.find((d: { terrain: string }) => d.terrain === 'Forest');
  assert.ok(move, 'starting forest can be reached by the settler');
  raw = engine.tryMove(raw, JSON.stringify(move.action), seat);
  if (view().stopMovement) raw = engine.tryMove(raw, JSON.stringify(view().stopMovement), seat);
  const settler = view().settlers[0];
  assert.ok(settler.foundAction);
  const blocked = JSON.parse(raw);
  blocked.actions_left = 0;
  const blockedView = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(blocked), seat), seat));
  assert.equal(blockedView.settlers[0].foundAction, null);
  assert.equal(blockedView.settlers[0].foundReason, 'No actions left');
  const before = JSON.parse(raw);
  const after = JSON.parse(engine.tryMove(raw, JSON.stringify(settler.foundAction), seat));
  assert.ok(after.players[seat].cities.some((c: { position: string }) => c.position === move.position));
  assert.ok(!(after.players[seat].units ?? []).some((u: { id: number }) => u.id === settler.id));
  assert.equal(after.actions_left, before.actions_left - 1);
});
