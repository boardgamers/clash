import { positionXY } from './model.ts';
import { seaEdgePoint } from './sea-routes.ts';
import type { Terrain } from './types';
type Point = [number, number];

// Reconstruct a visual route from public terrain; never invent a straight shortcut
// across an incompatible tile. Navigation uses the same perimeter as the sea guide.
export function movementWaypoints(tiles: [string, Terrain][], start: string, end: string, ship: boolean, navigation: string[][] = []): Point[] {
  if (start === end) return [positionXY(start)];
  const terrain = new Map(tiles);
  const sea = ship || (terrain.get(start) === 'Water' && terrain.get(end) === 'Water');
  const nodes = tiles.filter(([p, t]) => p === start || p === end || (sea ? t === 'Water' : t !== 'Water' && t !== 'Unexplored')).map(([p]) => p);
  const links = new Map<string, {to: string; points: Point[]; length: number}[]>();
  const add = (from: string, to: string, points: Point[]) => {
    const length = points.slice(1).reduce((sum, point, i) => sum + Math.hypot(point[0] - points[i][0], point[1] - points[i][1]), 0);
    links.set(from, [...(links.get(from) ?? []), {to, points, length}]);
  };
  for (const a of nodes) for (const b of nodes) {
    const pa = positionXY(a), pb = positionXY(b);
    if (Math.abs(Math.hypot(pa[0]-pb[0], pa[1]-pb[1])-Math.sqrt(3)) < .001) add(a,b,[pa,pb]);
  }
  if (sea) for (const path of navigation) {
    const a = path[0], b = path.at(-1);
    if (!a || !b || !nodes.includes(a) || !nodes.includes(b)) continue;
    const points: Point[] = [positionXY(a), ...path.map(p => seaEdgePoint(p,tiles)), positionXY(b)];
    add(a,b,points); add(b,a,[...points].reverse());
  }
  const distance = new Map([[start,0]]), routes = new Map<string,Point[]>([[start,[positionXY(start)]]]), pending = new Set(nodes);
  while (pending.size) {
    const current = [...pending].sort((a,b)=>(distance.get(a)??Infinity)-(distance.get(b)??Infinity))[0];
    if (!distance.has(current)) break;
    if (current === end) return routes.get(current)!;
    pending.delete(current);
    for (const link of links.get(current) ?? []) {
      const next = distance.get(current)! + link.length;
      if (pending.has(link.to) && next < (distance.get(link.to)??Infinity)) {
        distance.set(link.to,next); routes.set(link.to,[...routes.get(current)!,...link.points.slice(1)]);
      }
    }
  }
  return [];
}
