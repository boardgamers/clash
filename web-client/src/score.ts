/** Victory points use whole, half and tenth points; hide f32 serialization noise. */
export function formatPoints(points: number): string {
  return Number(points.toFixed(1)).toString();
}
