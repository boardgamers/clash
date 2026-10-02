import type { Player } from './types.ts';

type Unit = NonNullable<Player['units']>[number];
export interface TileUnitGroup {
  type: Unit['unit_type'];
  name: string;
  count: number;
  aboard: boolean;
  pirate: boolean;
}

export function tileUnitStacks(players: Player[], position: string) {
  return players.flatMap((player) => {
    const units = new Map<number, { unit: Pick<Unit, 'unit_type' | 'pirate'>; aboard: boolean }>();
    for (const unit of player.units ?? []) {
      if (unit.position !== position) continue;
      const aboard = unit.carrier_id != null || units.get(unit.id)?.aboard === true;
      units.set(unit.id, { unit, aboard });
      for (const passenger of unit.carried_units ?? [])
        units.set(passenger.id, { unit: passenger, aboard: true });
    }
    const groups = new Map<string, TileUnitGroup>();
    for (const { unit, aboard } of units.values()) {
      const pirate = unit.unit_type === 'Ship' && (unit.pirate === true || player.civilization === 'Pirates');
      const name = pirate ? 'Pirate ship' : typeof unit.unit_type === 'object' ? 'Leader' : unit.unit_type;
      const key = `${aboard}:${name}`;
      const group = groups.get(key) ?? { type: unit.unit_type, name, count: 0, aboard, pirate };
      group.count++;
      groups.set(key, group);
    }
    return groups.size ? [{ player, groups: [...groups.values()] }] : [];
  });
}
