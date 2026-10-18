import type { Pile } from './types.ts';
export interface CombatEstimate {
  samples: number;
  wins: number;
  draws: number;
  losses: number;
  ownLosses: number;
  enemyLosses: number;
  cost: Pile;
}
export interface CombatScenario {
  label: string;
  siege: number;
  steel: boolean;
}
export function addEstimate(a: CombatEstimate | undefined, b: CombatEstimate): CombatEstimate {
  if (!a) return b;
  return {
    samples: a.samples + b.samples,
    wins: a.wins + b.wins,
    draws: a.draws + b.draws,
    losses: a.losses + b.losses,
    ownLosses: a.ownLosses + b.ownLosses,
    enemyLosses: a.enemyLosses + b.enemyLosses,
    cost: b.cost,
  };
}
export function percentages(e: CombatEstimate): number[] {
  const values = [e.wins, e.draws, e.losses].map((n) => (n * 100) / e.samples);
  const rounded = values.map(Math.floor);
  const order = values
    .map((n, i) => ({ i, fraction: n - rounded[i] }))
    .sort((a, b) => b.fraction - a.fraction);
  const remaining = 100 - rounded.reduce((a, b) => a + b, 0);
  for (let i = 0; i < remaining; i++) rounded[order[i].i]++;
  return rounded;
}

export interface EstimateResult {
  results: (CombatEstimate | undefined)[];
  errors: (string | undefined)[];
}
const cache = new Map<string, EstimateResult>();
export function cachedEstimate(key: string) {
  return cache.get(key);
}
export function cacheEstimate(key: string, result: EstimateResult) {
  cache.delete(key);
  cache.set(key, result);
  while (cache.size > 4) cache.delete(cache.keys().next().value!);
}
