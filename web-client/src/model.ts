import type { Pile, Resource } from './types.ts';
import { resourceNames } from './types.ts';
export function pileText(pile: Pile): string {
  return (
    Object.entries(pile)
      .filter(([, n]) => n)
      .map(([key, n]) => `${n} ${resourceNames[key as Resource].toLowerCase()}`)
      .join(' + ') || 'No resources'
  );
}
export { journal } from './journal.ts';
// Turn availability is communicated once by the toolbar; keep item-specific restrictions.
export function actionReason(reason: string | null | undefined): string {
  if (!reason || reason === 'No actions left' || reason.startsWith('Wait for your turn')) return '';
  return reason;
}
type TextPart = { text: string; resource?: Resource; position?: string };
export function journalParts(text: string, namedResources = false): TextPart[] {
  const names: Record<string, Resource> = {
    food: 'food',
    wood: 'wood',
    ore: 'ore',
    idea: 'ideas',
    ideas: 'ideas',
    gold: 'gold',
    mood: 'mood_tokens',
    'mood token': 'mood_tokens',
    'mood tokens': 'mood_tokens',
    culture: 'culture_tokens',
    captive: 'captives',
    captives: 'captives',
    'culture token': 'culture_tokens',
    'culture tokens': 'culture_tokens',
  };
  const pattern = namedResources
    ? /\b(?:\d+(?:\.\d+)? )?(food|wood|ore|ideas?|gold|mood tokens?|culture tokens?|captives?)\b|\b([A-Z]\d+)\b/gi
    : /\b\d+(?:\.\d+)? (food|wood|ore|ideas?|gold|mood(?: tokens?)?|culture(?: tokens?)?|captives?)\b|\b([A-Z]\d+)\b/g;
  const parts: TextPart[] = [];
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > offset) parts.push({ text: text.slice(offset, match.index) });
    parts.push(
      match[2]
        ? { text: match[0], position: match[2] }
        : { text: match[0], resource: names[match[1].toLowerCase()] },
    );
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
