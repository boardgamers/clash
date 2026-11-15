import assert from 'node:assert/strict';
import test from 'node:test';
import { ViewerEmitter } from '@boardgamers/protocol/viewer';
import { viewerPreferencesSchema } from '@boardgamers/protocol';
import { hostPreferences } from '../scripts/host-preferences-fixture.mjs';

test('inlined platform badges reject the entire preferences snapshot at the reported paths', () => {
  const input = hostPreferences();
  for (const [seat, badge] of [
    [0, 0],
    [1, 1],
    [2, 0],
  ]) {
    input.bgs.players[seat].badges[badge].url = 'data:image/svg+xml,%3Csvg%3E%3C/svg%3E';
  }
  const parsed = viewerPreferencesSchema.safeParse(input);
  assert.equal(parsed.success, false);
  if (!parsed.success)
    assert.deepEqual(
      parsed.error.issues.map((issue) => issue.path),
      [
        ['bgs', 'players', 0, 'badges', 0, 'url'],
        ['bgs', 'players', 1, 'badges', 1, 'url'],
        ['bgs', 'players', 2, 'badges', 0, 'url'],
      ],
    );
  const diagnostics: unknown[] = [];
  const emitter = new ViewerEmitter({ onInvalid: (d) => diagnostics.push(d) });
  let delivered = false;
  emitter.on('preferences', () => {
    delivered = true;
  });
  assert.equal(emitter.emit('preferences', input), false);
  assert.equal(delivered, false, 'homeAtBottom never reaches the viewer');
  assert.equal(diagnostics.length, 1);
  assert.match(JSON.stringify(diagnostics), /https/);
});

test('HTTP asset badges deliver the complete preferences snapshot, including homeAtBottom', () => {
  const input = hostPreferences();
  const diagnostics: unknown[] = [];
  const emitter = new ViewerEmitter({ onInvalid: (d) => diagnostics.push(d) });
  let delivered: unknown;
  emitter.on('preferences', (value) => {
    delivered = value;
  });
  assert.equal(emitter.emit('preferences', input), true);
  assert.deepEqual(delivered, input);
  assert.deepEqual(diagnostics, []);
});

test('badge URL validation remains strict; unsafe and relative URLs are not accepted', () => {
  for (const url of [
    '/badge.svg',
    '//example.test/badge.svg',
    'data:image/svg+xml,<svg/>',
    'javascript:alert(1)',
  ]) {
    const input = hostPreferences();
    input.bgs.players[0].badges[0].url = url;
    assert.equal(viewerPreferencesSchema.safeParse(input).success, false);
  }
});
