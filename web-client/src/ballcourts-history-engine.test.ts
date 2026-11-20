import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
const fixture = readFileSync(
  new URL('../../server/tests/fixtures/legacy_ballcourts_history.json', import.meta.url),
  'utf8',
);
const expected = JSON.parse(
  readFileSync(
    new URL('../../server/tests/fixtures/legacy_ballcourts_history.expected.json', import.meta.url),
    'utf8',
  ),
);
const move = (raw: string, action: unknown) => engine.tryMove(raw, JSON.stringify(action), 0);
function matches(raw: string, projection: any) {
  const data = JSON.parse(raw);
  assert.equal(data.events?.length ?? 0, 0, 'no newly introduced payment decision');
  for (const [key, value] of Object.entries(projection)) assert.deepEqual(data[key], value, key);
}
test('0.4.160 Ballcourts history replays immediate mood payment and the following action', () => {
  matches(engine.replay(fixture, { to: expected.afterPayment }), expected.paid);
  matches(engine.replay(fixture, {}), expected.following);
  assert.equal(engine.logLength(engine.replay(fixture, {})), expected.full);
});
test('partial history branches retain the immediate Ballcourts action and replay identically', () => {
  const before = engine.replay(fixture, { to: expected.before });
  const branch = move(before, expected.bonus);
  matches(branch, expected.paid);
  matches(engine.replay(branch, {}), expected.paid);
});
test('old Ballcourts and following action undo/redo without extra payment responses', () => {
  const full = engine.replay(fixture, {});
  const paid = move(full, 'Undo');
  matches(paid, expected.paid);
  matches(move(paid, 'Redo'), expected.following);
  const before = move(paid, 'Undo');
  matches(move(before, 'Redo'), expected.paid);
  const branch = move(before, expected.bonus);
  matches(branch, expected.paid);
  matches(move(move(branch, 'Undo'), 'Redo'), expected.paid);
  matches(engine.replay(branch, {}), expected.paid);
});
