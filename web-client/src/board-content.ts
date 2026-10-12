import type { Session, Terrain } from './types.ts';

/** Only data consumed when constructing map models and permanent labels.
 * Resources, advances and action availability update controls, not map geometry.
 */
export function boardContentSignature(s: Session, tiles: [string, Terrain][]): string {
  return JSON.stringify([
    tiles,
    s.game?.players.map(p => [
      p.id, p.civilization,
      p.cities?.map(c => [c.position, c.port_position, c.mood_state, c.city_pieces]),
      p.units?.map(u => [u.id, u.position, u.unit_type, u.pirate, u.carrier_id, u.carried_units]),
    ]),
    s.view?.players.map(p => [p.index, p.capital]),
    s.seat, s.colorBlind, s.playerColors, s.playerSymbols, s.strategyMap,
  ]);
}
