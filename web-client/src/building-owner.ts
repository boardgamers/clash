import type { Player } from './types';

type CivilizationOwner = Pick<Player, 'id' | 'civilization'>;

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
