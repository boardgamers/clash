import type { ChatMessage, ChatMention } from '@boardgamers/protocol/chat';
import { playerColor, type Player, type Session } from './types.ts';

export function chatPlayerIndex(
  message: ChatMessage,
  players: readonly Pick<Player, 'id' | 'name'>[],
  mentions: readonly ChatMention[] = [],
): number | undefined {
  if (message.type === 'system') return undefined;
  if (message.playerIndex !== undefined) return message.playerIndex;
  const mentioned = message.authorId && mentions.find((person) => person.id === message.authorId);
  if (mentioned && mentioned.playerIndex !== undefined) return mentioned.playerIndex;
  const matches = message.author ? players.filter((player) => player.name === message.author) : [];
  return matches.length === 1 ? matches[0].id : undefined;
}

function rgb(hex: string) {
  return [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16));
}
function luminance(channels: number[]) {
  return channels.reduce((total, channel, index) => {
    const s = channel / 255;
    return (
      total + (s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][index]
    );
  }, 0);
}

/** Keep the board hue, shading names for contrast against the chat's paper. */
export function chatPlayerColor(
  index: number,
  preferences: Pick<Session, 'dark' | 'colorBlind' | 'playerColors'>,
): string {
  const original = rgb(playerColor(index, preferences.colorBlind, preferences.playerColors));
  const background = luminance(rgb(preferences.dark ? '#1c2c38' : '#f7f9fa'));
  const target = preferences.dark ? 255 : 0;
  for (let step = 0; step <= 20; step++) {
    const adjusted = original.map((channel) => Math.round(channel + ((target - channel) * step) / 20));
    const foreground = luminance(adjusted);
    if ((Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05) >= 4.5)
      return '#' + adjusted.map((channel) => channel.toString(16).padStart(2, '0')).join('');
  }
  return preferences.dark ? '#ffffff' : '#000000';
}
