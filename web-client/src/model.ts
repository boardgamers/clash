import type { Game, JournalEntry, LoggedAction, Pile, Resource } from './types.ts';
import { resourceNames } from './types.ts';
export function pileText(pile: Pile): string {
  return (
    Object.entries(pile)
      .filter(([, n]) => n)
      .map(([key, n]) => `${n} ${resourceNames[key as Resource].toLowerCase()}`)
      .join(' + ') || 'No resources'
  );
}
export function journal(game: Game): JournalEntry[] {
  return (game.log ?? []).flatMap((age, a) =>
    age.rounds.flatMap((round, r) =>
      round.turns.flatMap((turn, t) =>
        (turn.actions ?? []).flatMap((action, c) => {
          if (turn.turn_type === 'Setup' || action.action === 'Setup') {
            return setupJournal(game, action).map((setup, i) => ({
              id: `${a}-${r}-${t}-${c}-${i}`,
              age: age.age,
              round: round.round,
              kind: 'setup' as const,
              setup,
              text: `${setup.player} plays as ${setup.civilization}${setup.position ? ` · starts at ${setup.position}` : ''}`,
            }));
          }
          return (action.log ?? []).map((text, i) => ({
            id: `${a}-${r}-${t}-${c}-${i}`,
            age: age.age,
            round: round.round,
            kind: journalKind(action),
            text,
          }));
        }),
      ),
    ),
  );
}
function setupJournal(game: Game, action: LoggedAction): NonNullable<JournalEntry['setup']>[] {
  const players = [...new Set((action.items ?? []).map((item) => item.player))];
  if (players.length) {
    return players.flatMap((id) => {
      const player = game.players.find((p) => p.id === id);
      if (!player) return [];
      // Read the original location from history, even if the city has since moved or been lost.
      const position = action.items?.find(
        (item) =>
          item.player === id &&
          item.Structure?.structure === 'CityCenter' &&
          item.Structure.balance === 'Gain',
      )?.Structure?.position;
      return [{ player: player.name ?? `Player ${id + 1}`, civilization: player.civilization, position }];
    });
  }
  // Older saved logs may contain only their rendered text.
  return (action.log ?? []).flatMap((text) => {
    const match = text.match(/^(.+?): Setup: Play as ([^,]+)/);
    if (!match) return [];
    return [
      { player: match[1], civilization: match[2], position: text.match(/\bGain city ([A-Z]+\d+)\b/)?.[1] },
    ];
  });
}
function journalKind(action: LoggedAction): JournalEntry['kind'] {
  if (action.combat_stats) return 'combat';
  if (
    action.items?.some(
      (item) =>
        item.HandCard?.to && typeof item.HandCard.to === 'object' && 'CompleteObjective' in item.HandCard.to,
    )
  )
    return 'objective';
  if (!action.action || typeof action.action === 'string') return 'event';
  if ('Movement' in action.action) return 'move';
  const playing = action.action.Playing;
  const name =
    typeof playing === 'string'
      ? playing
      : playing && typeof playing === 'object'
        ? Object.keys(playing)[0]
        : '';
  const kinds: Record<string, JournalEntry['kind']> = {
    Collect: 'collect',
    Advance: 'research',
    EndTurn: 'end-turn',
    Construct: 'build',
    FoundCity: 'build',
    Recruit: 'recruit',
    ActionCard: 'card',
    WonderCard: 'card',
  };
  return kinds[name] ?? 'event';
}
export function journalParts(text: string): { text: string; resource?: Resource }[] {
  const names: Record<string, Resource> = {
    food: 'food',
    wood: 'wood',
    ore: 'ore',
    ideas: 'ideas',
    gold: 'gold',
    'mood token': 'mood_tokens',
    'mood tokens': 'mood_tokens',
    'culture token': 'culture_tokens',
    'culture tokens': 'culture_tokens',
  };
  const pattern = /\b\d+(?:\.\d+)? (food|wood|ore|ideas|gold|mood tokens?|culture tokens?)\b/g;
  const parts: { text: string; resource?: Resource }[] = [];
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > offset) parts.push({ text: text.slice(offset, match.index) });
    parts.push({ text: match[0], resource: names[match[1]] });
    offset = match.index + match[0].length;
  }
  if (offset < text.length) parts.push({ text: text.slice(offset) });
  return parts;
}
export function positionXY(position: string): [number, number] {
  const q = position.charCodeAt(0) - 65;
  const row = Number(position.slice(1)) - 1;
  return [q * 1.5, (row + (q % 2) * 0.5) * Math.sqrt(3)];
}
