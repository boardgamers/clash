import { positionXY } from './model';

/** Face the wharf out from its city, in the designated water hex at the shared shore. */
export function portPlacement(city: string, water: string) {
  const [cx, cz] = positionXY(city),
    [wx, wz] = positionXY(water);
  const dx = wx - cx,
    dz = wz - cz,
    distance = Math.hypot(dx, dz);
  if (Math.abs(distance - Math.sqrt(3)) > 0.01) return null;
  return { x: wx - (dx / distance) * 0.7, z: wz - (dz / distance) * 0.7, rotation: Math.atan2(dx, dz) };
}
