// Saved games retain the original IDs; only visible names use Monumental Edition wording.
export function officialWonderText(text: string): string {
  return text
    .replace(/\bColosseum\b/g, 'Great Arena')
    .replace(/\b(?:Great Pyramid(?:s)?|Pyramids)\b/g, 'Great Pyramids');
}

export function wonderName(id: string): string {
  return officialWonderText(id.replace(/([a-z])([A-Z])/g, '$1 $2'));
}
