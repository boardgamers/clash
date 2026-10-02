import type { Game, Session } from './types.ts';

// Public block placement only. Hidden terrain is never needed for this outline.
export function unexploredRegion(map: Game['map'], destination: string): string[] {
  if (!map.tiles.some(([p, t]) => p === destination && t === 'Unexplored')) return [];
  for (const { position } of map.unexplored_blocks ?? []) {
    const q = position.top_tile.charCodeAt(0) - 65;
    const r = Number(position.top_tile.slice(1)) - 1 - Math.floor(q / 2);
    // Opposite rotations occupy the same four hexes (see server map::Block::tiles).
    const rotation = position.rotation % 3;
    const positions = [
      [0, 0],
      [-1, 1],
      [1, 0],
      [0, 1],
    ].map(([x, y]) => {
      const [dq, dr] = rotation === 0 ? [x, y] : rotation === 1 ? [x + y, -x] : [y, -x - y];
      const col = q + dq;
      return String.fromCharCode(65 + col) + (r + dr + Math.floor(col / 2) + 1);
    });
    if (positions.includes(destination)) return positions;
  }
  return [];
}

export function explorationPreview(
  s: Pick<Session, 'game' | 'mode' | 'moveTarget' | 'moveDestinations' | 'playback'>,
): string[] {
  if (
    !s.game ||
    s.playback ||
    s.mode !== 'settlers' ||
    !s.moveTarget ||
    !s.moveDestinations.some((d) => d.position === s.moveTarget)
  )
    return [];
  return unexploredRegion(s.game.map, s.moveTarget);
}
