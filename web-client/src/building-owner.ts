import type { City, Player } from './types.ts';

type CivilizationOwner = Pick<Player, 'id' | 'civilization'>;

export interface CityInfluencer {
  owner: number;
  civilization: string;
  count: number;
  buildings: string[];
}

/** Use the piece owner's normal architecture, including NPCs; tolerate stale owner IDs. */
export function buildingCivilization(
  owner: unknown,
  cityOwner: CivilizationOwner,
  players: readonly CivilizationOwner[],
): string {
  return players.find((player) => player.id === owner)?.civilization ?? cityOwner.civilization;
}

/** Only foreign pieces need an owner suffix in city hover details. */
export function foreignBuildingCivilization(
  owner: unknown,
  cityOwner: CivilizationOwner,
  players: readonly CivilizationOwner[],
): string | undefined {
  return owner === cityOwner.id ? undefined : players.find((player) => player.id === owner)?.civilization;
}

/** Group foreign buildings in a player or Barbarian city by influencing player. */
export function cityInfluencers(
  city: Pick<City, 'city_pieces'> | null | undefined,
  cityOwner: number,
  players: readonly CivilizationOwner[],
): CityInfluencer[] {
  const byOwner = new Map<number, CityInfluencer>();
  for (const [key, owner] of Object.entries(city?.city_pieces ?? {})) {
    if (key === 'wonders' || typeof owner !== 'number' || !Number.isFinite(owner) || owner === cityOwner) continue;
    const player = players.find((entry) => entry.id === owner);
    if (!player) continue;
    const building = key[0].toUpperCase() + key.slice(1);
    const existing = byOwner.get(owner);
    if (existing) {
      existing.count++;
      existing.buildings.push(building);
    } else {
      byOwner.set(owner, { owner, civilization: player.civilization, count: 1, buildings: [building] });
    }
  }
  const order = new Map(players.map((player, index) => [player.id, index]));
  return [...byOwner.values()].sort(
    (a, b) => b.count - a.count || (order.get(a.owner) ?? a.owner) - (order.get(b.owner) ?? b.owner),
  );
}

/** Concise accessible summary for hover titles and screen-reader labels. */
export function cityInfluenceSummary(
  influencers: readonly Pick<CityInfluencer, 'civilization' | 'count'>[],
): string {
  if (!influencers.length) return '';
  const summary = influencers
    .map((entry) => (entry.count > 1 ? `${entry.civilization} (${entry.count})` : entry.civilization))
    .join(', ');
  return `Influenced by ${summary}`;
}

function srgbChannel(hexByte: number) {
  const value = hexByte / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

/** Match CivilizationFlags luminance contrast so medallion emblems stay legible on any player color. */
export function emblemInkColor(color: string): string {
  const hex = color.trim().replace(/^#/, '');
  const full = hex.length === 3 ? hex.replace(/./g, '$&$&') : hex;
  if (!/^[0-9a-f]{6}$/i.test(full)) return '#fff4dc';
  const r = srgbChannel(parseInt(full.slice(0, 2), 16));
  const g = srgbChannel(parseInt(full.slice(2, 4), 16));
  const b = srgbChannel(parseInt(full.slice(4, 6), 16));
  const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return luminance > 0.32 ? '#293b37' : '#fff4dc';
}
