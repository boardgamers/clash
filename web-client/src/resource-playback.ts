import { publicActions, playingAction } from './replay-actions.ts';
import { resources, resourceNames, type BoardFrame, type Game, type Pile, type Resource } from './types.ts';

export interface ResourceMarker {
  player: number;
  position: string;
  resource: Resource;
  amount: number;
}
export interface ResourceGain {
  player: number;
  pile: Pile;
  waste: Pile;
}
type Collect = { city_position: string; collections: { position: string; pile: Pile; times: number }[] };
const add = (to: Pile, from: Pile) => {
  for (const r of resources) if (from[r]) to[r] = (to[r] ?? 0) + from[r]!;
};
export const describeResources = (pile: Pile) =>
  resources
    .filter((r) => (pile[r] ?? 0) > 0)
    .map((r) => `${pile[r]} ${r === 'ideas' && pile[r] === 1 ? 'idea' : resourceNames[r].toLowerCase()}`)
    .join(', ');

/** Use public recorded gains and historical cities; never today's balances or private choices. */
export function frameResources(
  game: Game,
  after: number,
  frame: BoardFrame,
): { gains: ResourceGain[]; markers: ResourceMarker[] } {
  const totals = new Map<number, ResourceGain>();
  const markers: ResourceMarker[] = [];
  let root: Record<string, unknown> | null = null;
  let rootPlayer: number | undefined;
  for (const [index, action] of publicActions(game).slice(0, frame.cursor).entries()) {
    const playing = playingAction(action);
    if (playing) {
      root = playing;
      rootPlayer = action.player ?? action.items?.[0]?.player;
    } else if (
      typeof action.action === 'string' ||
      (typeof action.action === 'object' && action.action && !('Response' in action.action))
    )
      root = null;
    const interrupted =
      action.items?.some((item) => item.CombatRound || item.Text?.startsWith('triggers the event ')) ||
      action.log?.some((line) => line.startsWith('A new game event has been triggered:'));
    if (index < after) {
      if (interrupted) root = null;
      continue;
    }
    const gains = new Map<number, ResourceGain>();
    for (const item of action.items ?? []) {
      if (!item.Resources) continue;
      const gain = gains.get(item.player) ?? { player: item.player, pile: {}, waste: {} };
      if (item.Resources.balance === 'Gain') add(gain.pile, item.Resources.resources);
      else if (item.origin?.Ability === 'Waste') add(gain.waste, item.Resources.resources);
      gains.set(item.player, gain);
    }
    for (const gain of gains.values()) {
      for (const r of resources) gain.pile[r] = Math.max(0, (gain.pile[r] ?? 0) - (gain.waste[r] ?? 0));
      const total = totals.get(gain.player) ?? { player: gain.player, pile: {}, waste: {} };
      add(total.pile, gain.pile);
      add(total.waste, gain.waste);
      totals.set(gain.player, total);
      const pool = { ...gain.pile };
      const cities = frame.players.find((p) => p.id === gain.player)?.cities ?? [];
      if (action.items?.some((item) => item.origin?.Incident !== undefined)) continue;
      const place = (position: string, resource: Resource, requested: number) => {
        const amount = Math.min(requested, pool[resource] ?? 0);
        if (amount <= 0 || !frame.tiles.some(([p]) => p === position)) return;
        markers.push({ player: gain.player, position, resource, amount });
        pool[resource] = pool[resource]! - amount;
      };
      const ownRoot = rootPlayer === undefined || rootPlayer === gain.player ? root : null;
      const custom = ownRoot?.Custom as { action?: string; city?: string } | undefined;
      const taxes =
        custom?.action === 'Taxes' ||
        action.items?.some(
          (i) =>
            i.player === gain.player &&
            i.Resources?.balance === 'Gain' &&
            (i.origin?.Ability === 'Taxes' || i.origin?.Advance === 'Taxes'),
        );
      if (taxes) {
        // Taxes grants a shared mix, one resource per city; distribute that actual mix for display.
        for (const city of [...cities].sort((a, b) => a.position.localeCompare(b.position))) {
          const resource = resources.find((r) => (pool[r] ?? 0) > 0);
          if (resource) place(city.position, resource, 1);
        }
        continue;
      }
      const collect = ownRoot?.Collect as Collect | undefined;
      if (collect)
        for (const tile of collect.collections)
          for (const r of resources) place(tile.position, r, (tile.pile[r] ?? 0) * tile.times);
      const city =
        collect?.city_position ??
        custom?.city ??
        (ownRoot?.Recruit as { city_position?: string } | undefined)?.city_position ??
        (ownRoot?.Construct as { city_position?: string } | undefined)?.city_position;
      if (city && cities.some((c) => c.position === city))
        for (const r of resources) place(city, r, pool[r] ?? 0);
    }
    if (interrupted) root = null;
  }
  const combined = new Map<string, ResourceMarker>();
  for (const marker of markers) {
    const key = `${marker.player}:${marker.position}:${marker.resource}`;
    const old = combined.get(key);
    combined.set(key, { ...marker, amount: (old?.amount ?? 0) + marker.amount });
  }
  return {
    gains: [...totals.values()].filter((g) => describeResources(g.pile) || describeResources(g.waste)),
    markers: [...combined.values()],
  };
}
