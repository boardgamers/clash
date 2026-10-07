import { positionXY } from './model.ts';

/**
 * Distance from the water hex's center to the wharf, toward the shared shore. The warehouse
 * roof rises above the shore, so the wharf keeps clear of buildings on the city's edge.
 */
const PORT_INSET = 0.62;

/** Face the wharf out from its city, in the designated water hex at the shared shore. */
export function portPlacement(city: string, water: string) {
  const [cx, cz] = positionXY(city),
    [wx, wz] = positionXY(water);
  const dx = wx - cx,
    dz = wz - cz,
    distance = Math.hypot(dx, dz);
  if (Math.abs(distance - Math.sqrt(3)) > 0.01) return null;
  return {
    x: wx - (dx / distance) * PORT_INSET,
    z: wz - (dz / distance) * PORT_INSET,
    rotation: Math.atan2(dx, dz),
  };
}
