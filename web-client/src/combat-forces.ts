import { activeCombatState } from './active-combat.ts';
import type { Game, Player } from './types.ts';

type Unit = NonNullable<Player['units']>[number];
export interface CombatForce {
  count: number;
  units: { type: Unit['unit_type']; count: number; label: string }[];
}

function summarize(units: Unit[]): CombatForce {
  const groups = new Map<string, CombatForce['units'][number]>();
  for (const unit of units) {
    // A generic leader label avoids consulting private card/leader catalogs.
    const label = typeof unit.unit_type === 'string' ? unit.unit_type : 'Leader';
    const group = groups.get(label) ?? { type: unit.unit_type, count: 0, label };
    group.count++;
    groups.set(label, group);
  }
  return { count: units.length, units: [...groups.values()] };
}

/** Current fighters, not the source stack or the original force before casualties.
 * Mirrors active_attackers/active_defenders: armies on land, ships at sea.
 * Missing visible participant data is unknown, never guessed from a tile stack.
 */
export function combatForces(game: Game | null) {
  if (!game) return null;
  const combat = activeCombatState(game);
  if (!combat) return null;
  const { stats } = combat;
  const terrain = game.map.tiles.find(([p]) => p === stats.defender.position)?.[1];
  const knownTerrain = terrain !== undefined && terrain !== 'Unexplored';
  const fighter = (u: Unit) =>
    terrain === 'Water'
      ? u.unit_type === 'Ship'
      : typeof u.unit_type === 'object' || ['Infantry', 'Cavalry', 'Elephant'].includes(u.unit_type);
  const attackers = game.players.find((p) => p.id === stats.attacker.player)?.units;
  const defenders = game.players.find((p) => p.id === stats.defender.player)?.units;
  const ids = combat.attackers;
  const selected = ids?.map((id) => attackers?.find((u) => u.id === id));
  return {
    round: stats.round,
    attacker:
      knownTerrain && selected && selected.every((u): u is Unit => !!u)
        ? summarize(selected.filter(fighter))
        : null,
    defender:
      knownTerrain && defenders
        ? summarize(defenders.filter((u) => u.position === stats.defender.position && fighter(u)))
        : null,
  };
}
