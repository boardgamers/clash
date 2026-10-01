import type { Game, View } from './types.ts';

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

/** Public advances include borrowed abilities, just as the engine's combat calculation does. */
export function steelWeaponsBenefit(game: Game | null, view: View | null, player?: number): string | null {
  const combat = game && activeCombat(game);
  if (!combat || !view) return null;
  const enemy =
    combat.attacker.player === player
      ? combat.defender.player
      : combat.defender.player === player
        ? combat.attacker.player
        : null;
  if (enemy === null) return null;
  const hasSteel = view.players.find((p) => p.index === enemy)?.advances.some((a) => a.id === 'SteelWeapons');
  return hasSteel ? '+1 combat value each round. Enemy has Steel Weapons.' : '+2 combat value each round.';
}
