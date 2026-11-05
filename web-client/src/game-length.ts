import type { Game } from './types.ts';

export function ageCount(game: Game | null) {
  switch (game?.options?.length) {
    case 'Epic':
      return 8;
    case 'Shorter':
      return 4;
    case 'RollToEnd':
    case 'WonderfulEnding':
      return 7;
    default:
      return 6;
  }
}
export function ageLabel(age: number) {
  return ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'][age - 1] ?? String(age);
}

export function variableEndThreshold(age: number, round: number): number | null {
  if (age === 6 && round === 2) return 6;
  if (age === 6 && round === 3) return 5;
  if (age === 7 && round >= 1 && round <= 3) return 5 - round;
  return null;
}

export function lengthRule(game: Game | null): string {
  switch (game?.options?.length) {
    case 'Shorter':
      return 'Shorter Game: 4 ages, 2 starting settlers. Between ages, take two free advances in two full turn-order passes; the second uses no event marker.';
    case 'RollToEnd':
      return 'Roll to End: roll at the start of each marked round, before any player acts. A matching result ends play immediately; only objectives follow.';
    case 'WonderfulEnding':
      return 'Starting from age 6, round 2, building a wonder ends the game after that round. Then complete objectives.';
    case 'Epic':
      return 'Epic: 8 ages.';
    default:
      return 'Normal: 6 ages.';
  }
}

export function lengthName(game: Game | null): string {
  switch (game?.options?.length) {
    case 'Shorter':
      return 'Shorter';
    case 'RollToEnd':
      return 'Roll to End';
    case 'WonderfulEnding':
      return 'Wonderful Ending';
    case 'Epic':
      return 'Epic';
    default:
      return 'Normal';
  }
}
