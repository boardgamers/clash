/** Translate complete sentences before restoring their interactive references.
 * References retain engine IDs; only visible labels are translated.
 */
export function localizedParts<T extends { text: string }>(
  source: string,
  parts: T[],
  translate: (text: string) => string,
): (T | { text: string })[] {
  const translated = translate(source);
  if (translated === source) return parts.map((part) => ({ ...part, text: translate(part.text) }));
  const labels = parts
    .filter((part) => Object.keys(part).some((key) => key !== 'text'))
    .map((part) => ({ part, label: translate(part.text) }));
  const result: (T | { text: string })[] = [];
  let offset = 0;
  while (offset < translated.length) {
    const candidates = labels
      .flatMap(({ part, label }) => {
        if (!label) return [];
        const pattern = new RegExp(label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'giu');
        for (const match of translated.slice(offset).matchAll(pattern)) {
          const index = offset + match.index;
          const end = index + match[0].length;
          const cjk = /[\p{Script=Han}\p{Script=Hangul}]/u.test(label);
          const word = /[\p{L}\p{N}_]/u;
          if (
            !cjk &&
            ((index > 0 && word.test(label[0]) && word.test(translated[index - 1])) ||
              (end < translated.length && word.test(label.at(-1)!) && word.test(translated[end])))
          )
            continue;
          return [{ part, label: match[0], index }];
        }
        return [];
      })
      .sort((a, b) => a.index - b.index || b.label.length - a.label.length);
    const next = candidates[0];
    if (!next) {
      result.push({ text: translated.slice(offset) });
      break;
    }
    if (next.index > offset) result.push({ text: translated.slice(offset, next.index) });
    result.push({ ...next.part, text: next.label });
    offset = next.index + next.label.length;
  }
  return result;
}
