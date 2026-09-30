import type { Game } from './types.ts';

export interface ActiveCombat {
  round: number;
  attacker: { position: string; player: number };
  defender: { position: string; player: number };
}

/** Combat may sit below another pending ability on the event stack. */
export function activeCombat(game: Pick<Game, 'events'>): ActiveCombat | null {
  for (const { event_type: event } of [...(game.events ?? [])].reverse()) {
    if (typeof event !== 'object' || event === null) continue;
    if ('CombatEnd' in event) return null;
    const combat = (event.CombatStart ??
      (event.CombatRoundStart as { combat?: unknown })?.combat ??
      (event.CombatRoundEnd as { combat?: unknown })?.combat) as { stats?: ActiveCombat } | undefined;
    const stats = combat?.stats;
    if (
      stats &&
      [stats.attacker, stats.defender].every(
        (side) => side && /^[A-Z]+\d+$/.test(side.position) && Number.isInteger(side.player),
      )
    )
      return stats;
  }
  return null;
}
