import { positionXY } from './model.ts';
import type { Terrain } from './types';

export function seaConnections(tiles: [string, Terrain][]): [string, string][] {
  const water = tiles.filter(([, terrain]) => terrain === 'Water').map(([p]) => p);
  return water.flatMap((from, i) =>
    water
      .slice(i + 1)
      .filter((to) => {
        const a = positionXY(from),
          b = positionXY(to);
        return Math.abs((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 - 3) < 0.001;
      })
      .map((to): [string, string] => [from, to]),
  );
}

export function seaArea(start: string | null, connections: [string, string][]): Set<string> {
  const area = new Set<string>(start ? [start] : []);
  for (const position of area) {
    for (const [a, b] of connections) {
      if (a === position) area.add(b);
      if (b === position) area.add(a);
    }
  }
  return area;
}

// Position the guide outside the exposed edge of each perimeter hex.
export function seaEdgePoint(position: string, tiles: [string, Terrain][]): [number, number] {
  const [x, z] = positionXY(position);
  const points = tiles.map(([p]) => positionXY(p));
  let dx = 0,
    dz = 0;
  for (let i = 0; i < 6; i++) {
    const angle = Math.PI / 6 + (i * Math.PI) / 3;
    const nx = Math.cos(angle),
      nz = Math.sin(angle);
    if (
      !points.some(([px, pz]) => Math.hypot(px - x - nx * Math.sqrt(3), pz - z - nz * Math.sqrt(3)) < 0.001)
    ) {
      dx += nx;
      dz += nz;
    }
  }
  const length = Math.hypot(dx, dz) || 1;
  return [x + (dx / length) * 1.35, z + (dz / length) * 1.35];
}
