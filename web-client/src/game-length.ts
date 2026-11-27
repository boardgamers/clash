import { activeHistory } from './active-history.ts';
import type { Game } from './types.ts';

export interface RollToEndResult {
  age: number;
  round: number;
  roll: number;
  threshold: number;
  ended: boolean;
}

const ROLL_TO_END_RE = /^Roll to End:\s*(\d+)\s*\(end on (\d+)(?:–6)?\)$/;

export function formatRollToEnd(roll: number, threshold: number): string {
  return threshold === 6
    ? `Roll to End: ${roll} (end on 6)`
    : `Roll to End: ${roll} (end on ${threshold}–6)`;
}

export function normalizeRollToEndText(text: string): string {
  const match = ROLL_TO_END_RE.exec(text);
  if (!match) return text;
  return formatRollToEnd(Number(match[1]), Number(match[2]));
}

export function formatRollToEndNote(text: string): string {
  const match = ROLL_TO_END_RE.exec(text);
  if (!match) return text;
  const roll = Number(match[1]);
  const threshold = Number(match[2]);
  return threshold === 6 ? `${roll} (end on 6)` : `${roll} (end on ${threshold}–6)`;
}

export function rollToEndRolls(game: Game | null): RollToEndResult[] {
  if (!game) return [];
  const results: RollToEndResult[] = [];
  for (const ageEntry of activeHistory(game)) {
    for (const roundEntry of ageEntry.rounds ?? []) {
      for (const turn of roundEntry.turns ?? []) {
        for (const action of turn.actions ?? []) {
          for (const item of action.items ?? []) {
            if (item.origin?.Ability !== 'Roll to End' || typeof item.Text !== 'string') continue;
            const match = ROLL_TO_END_RE.exec(item.Text);
            if (!match) continue;
            const roll = Number(match[1]);
            const threshold = Number(match[2]);
            results.push({
              age: ageEntry.age,
              round: roundEntry.round,
              roll,
              threshold,
              ended: roll >= threshold,
            });
          }
        }
      }
    }
  }
  return results;
}

export function currentRollToEnd(
  game: Game | null,
  age: number,
  round: number,
): RollToEndResult | null {
  const rolls = rollToEndRolls(game);
  for (let i = rolls.length - 1; i >= 0; i--) {
    if (rolls[i].age === age && rolls[i].round === round) return rolls[i];
  }
  if (game?.variable_end === 'RollToEnd') {
    for (let i = rolls.length - 1; i >= 0; i--) {
      if (rolls[i].ended) return rolls[i];
    }
  }
  return null;
}

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
