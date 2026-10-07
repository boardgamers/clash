import type { Game, Player } from './types.ts';

export function militarySummary(player: Player, position?: string) {
  const seen = new Set<number>();
  const result = { army: 0, aboard: 0, ships: 0, settlers: 0 };
  const count = (unit: { id: number; unit_type: string | { Leader: string } }, aboard: boolean) => {
    if (seen.has(unit.id)) return;
    seen.add(unit.id);
    if (unit.unit_type === 'Ship') result.ships++;
    else if (unit.unit_type === 'Settler') result.settlers++;
    else if (
      typeof unit.unit_type === 'object' ||
      ['Infantry', 'Cavalry', 'Elephant'].includes(unit.unit_type)
    ) {
      result.army++;
      if (aboard) result.aboard++;
    }
  };
  for (const unit of player.units ?? []) {
    if (position && unit.position !== position) continue;
    count(unit, unit.carrier_id != null);
    for (const passenger of unit.carried_units ?? []) count(passenger, true);
  }
  return result;
}

export function strategyTiles(game: Pick<Game, 'map' | 'players'>) {
  return game.map.tiles
    .filter(([, terrain]) => terrain !== 'Unexplored')
    .map(([position, terrain]) => {
      const occupants = game.players.flatMap((player) => {
        const city = player.cities?.find((c) => c.position === position);
        const forces = militarySummary(player, position);
        if (!city && !forces.army && !forces.ships && !forces.settlers) return [];
        const size = city
          ? 1 +
            Object.values(city.city_pieces ?? {}).reduce<number>(
              (n, v) => n + (Array.isArray(v) ? v.length : typeof v === 'number' ? 1 : 0),
              0,
            )
          : 0;
        return [{ player, city, size, ...forces }];
      });
      return { position, terrain, occupants };
    });
}
export type StrategyTile = ReturnType<typeof strategyTiles>[number];

export function strategyDescription(tile: StrategyTile) {
  const terrain =
    typeof tile.terrain === 'string' ? tile.terrain : `Exhausted ${tile.terrain.Exhausted.toLowerCase()}`;
  return [
    terrain,
    ...tile.occupants.map((o) =>
      [
        o.player.civilization,
        o.city ? `city size ${o.size}${o.city.city_pieces?.fortress != null ? ', Fortress' : ''}` : '',
        o.city?.mood_state ?? '',
        o.army ? `${o.army} army${o.aboard ? ` (${o.aboard} aboard)` : ''}` : '',
        o.ships ? `${o.ships} ships` : '',
        o.settlers ? `${o.settlers} settlers` : '',
      ]
        .filter(Boolean)
        .join(' · '),
    ),
  ].join(' · ');
}
