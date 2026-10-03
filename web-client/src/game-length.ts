import type { Game } from './types.ts';

export function ageCount(game: Game | null) {
  return game?.options?.length === 'Epic' ? 10 : 6;
}
export function ageLabel(age: number) {
  return ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'][age - 1] ?? String(age);
}
