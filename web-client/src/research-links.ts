import type { AdvanceView, View } from './types.ts';

export interface ResearchReference {
  id: string;
  name: string;
  player?: number;
  civilization?: boolean;
}

export function researchReferences(view: View | null, player?: number): ResearchReference[] {
  if (!view) return [];
  const references = new Map<string, ResearchReference>();
  const add = (reference: ResearchReference) => {
    const key = reference.name.toLowerCase();
    if (!references.has(key)) references.set(key, reference);
  };
  for (const advance of view.advances) add({ id: advance.id, name: advance.name, player });
  // Spectator views also expose researched advances through the player overview.
  for (const p of view.players) {
    for (const advance of p.advances) {
      if (!p.civilizationAdvances.some((a) => a.id === advance.id))
        add({ id: advance.id, name: advance.name, player });
    }
  }
  for (const p of view.players.filter((p) => player === undefined || p.index === player))
    for (const advance of p.civilizationAdvances)
      add({ id: advance.id, name: advance.name, player: p.index, civilization: true });
  return [...references.values()];
}

export function researchTextParts(
  text: string,
  references: ResearchReference[],
): { text: string; research?: ResearchReference }[] {
  return namedTextParts(text, references).map(({ text, reference }) =>
    reference ? { text, research: reference } : { text },
  );
}

export function namedTextParts<T extends { name: string }>(
  text: string,
  references: T[],
): { text: string; reference?: T }[] {
  if (!references.length) return [{ text }];
  const names = new Map(references.map((reference) => [reference.name.toLowerCase(), reference]));
  const alternatives = [...names.keys()]
    .sort((a, b) => b.length - a.length)
    .map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/\s+/g, '\\s+'));
  const pattern = new RegExp(`(?<![\\p{L}\\p{N}_])(?:${alternatives.join('|')})(?![\\p{L}\\p{N}_])`, 'giu');
  const parts: { text: string; reference?: T }[] = [];
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > offset) parts.push({ text: text.slice(offset, match.index) });
    parts.push({ text: match[0], reference: names.get(match[0].toLowerCase().replace(/\s+/g, ' ')) });
    offset = match.index + match[0].length;
  }
  if (offset < text.length) parts.push({ text: text.slice(offset) });
  return parts;
}

export function researchFreeHints(advance: AdvanceView, advances: AdvanceView[]) {
  if (advance.owned || advance.borrowed || advance.costAmount === 0) return [];
  const sources = [
    ...(['Engineering', 'Roads'].includes(advance.id) ? ['Math'] : []),
    ...(['Navigation', 'Cartography'].includes(advance.id) ? ['Astronomy'] : []),
    ...(advance.group === 'Science' ? ['Priesthood'] : []),
  ];
  return sources.flatMap((id) => {
    const source = advances.find((a) => a.id === id);
    return source && !source.owned && !source.borrowed
      ? [{ id, name: source.name, oncePerTurn: id === 'Priesthood' }]
      : [];
  });
}
