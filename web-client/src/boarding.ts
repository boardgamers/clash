import type { MoveDestination } from './types';
export interface BoardingGroup { carrier: number; units: number[] }
// Ask the engine about each partial load: this includes existing passengers,
// civilization capacity bonuses and every selected unit's movement restrictions.
export function boardingPlan(ids: number[], position: string,
  query: (ids: number[]) => MoveDestination[], forced?: { unit: number; carrier: number }): BoardingGroup[] | null {
  const options = new Map(ids.map(id => [id, query([id]).filter(d => d.position === position && d.carrier != null && !Object.values(d.payment).some(Boolean))]));
  const groups: BoardingGroup[] = [];
  const ordered = [...ids].sort((a,b) => Number(b === forced?.unit) - Number(a === forced?.unit));
  const cache = new Map<string, boolean>();
  const failed = new Set<string>();
  function assign(index: number): boolean {
    if (index === ordered.length) return true;
    const state = index + ':' + groups.map(g => g.carrier + '=' + [...g.units].sort((a,b)=>a-b).join(',')).sort().join(';');
    if (failed.has(state)) return false;
    const id = ordered[index];
    for (const offer of options.get(id) ?? []) {
      const carrier = offer.carrier!;
      if (forced?.unit === id && forced.carrier !== carrier) continue;
      const existing = groups.find(g => g.carrier === carrier);
      const units = [...(existing?.units ?? []), id];
      const key = carrier + ':' + [...units].sort((a,b)=>a-b).join(',');
      if (!cache.has(key)) cache.set(key, query(units).some(d => d.position === position && d.carrier === carrier && !Object.values(d.payment).some(Boolean)));
      if (!cache.get(key)) continue;
      if (existing) existing.units.push(id); else groups.push({carrier, units:[id]});
      if (assign(index+1)) return true;
      if (existing) existing.units.pop(); else groups.pop();
    }
    failed.add(state);
    return false;
  }
  return assign(0) ? groups : null;
}
