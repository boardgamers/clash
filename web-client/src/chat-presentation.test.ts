import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chatPlayerColor, chatPlayerIndex } from './chat-presentation.ts';

const players = [
  { id: 0, name: 'Leif' },
  { id: 1, name: 'Aurelia' },
];

test('chat uses stable player identity before falling back to names', () => {
  const message = { type: 'text' as const, text: 'Hello', author: 'Leif' };
  assert.equal(chatPlayerIndex({ ...message, playerIndex: 1 }, players), 1);
  assert.equal(chatPlayerIndex({ ...message, playerIndex: 0 }, players), 0);
  assert.equal(
    chatPlayerIndex({ ...message, authorId: 'renamed' }, players, [
      { id: 'renamed', name: 'Aurelia', playerIndex: 1 },
    ]),
    1,
  );
  assert.equal(chatPlayerIndex(message, players), 0);
  assert.equal(chatPlayerIndex({ ...message, author: 'Spectator' }, players), undefined);
  assert.equal(chatPlayerIndex(message, [...players, { id: 2, name: 'Leif' }]), undefined);
});

test('system events never acquire a player identity', () => {
  assert.equal(
    chatPlayerIndex({ type: 'system', text: 'Game ended', author: 'Leif', playerIndex: 0 }, players),
    undefined,
  );
  assert.equal(chatPlayerIndex({ type: 'text', text: 'Game ended', author: 'Leif' }, players), 0);
});

function luminance(hex: string) {
  const channels = hex
    .slice(1)
    .match(/../g)!
    .map((value) => parseInt(value, 16) / 255)
    .map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}
for (const dark of [false, true]) {
  test(`chat names stay distinct and readable in ${dark ? 'dark' : 'light'} mode, including custom palettes`, () => {
    const background = luminance(dark ? '#1c2c38' : '#f7f9fa');
    for (const colorBlind of [false, true]) {
      const normal = Array.from({ length: 4 }, (_, seat) => chatPlayerColor(seat, { dark, colorBlind }));
      assert.equal(new Set(normal).size, 4);
      for (const color of [...normal, '#ffffff', '#000000', '#ffff00', '#223344', '#da73aa']) {
        const adjusted = chatPlayerColor(0, { dark, colorBlind, playerColors: [color] });
        const foreground = luminance(adjusted);
        assert.ok(
          (Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05) >= 4.5,
        );
      }
    }
    assert.notEqual(
      chatPlayerColor(0, { dark, colorBlind: false, playerColors: ['#da73aa'] }),
      chatPlayerColor(0, { dark, colorBlind: false }),
    );
    assert.equal(
      chatPlayerColor(0, { dark, colorBlind: true, playerColors: ['#ffffff'] }),
      chatPlayerColor(0, { dark, colorBlind: true }),
    );
  });
}
