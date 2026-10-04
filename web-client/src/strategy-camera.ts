import type { Game } from './types.ts';
import { positionXY } from './model.ts';

/** Use the original public home city, even if it has moved or been captured. */
export function strategyHome(game: Game, seat?: number): string | undefined {
  const player = game.players.find((p) => p.id === seat);
  if (!player) return;
  const names = [player.name, player.civilization, `Player${player.id + 1}`, `Player ${player.id + 1}`];
  for (const age of game.log ?? []) {
    if (age.age !== 0) continue;
    for (const round of age.rounds)
      for (const turn of round.turns)
        for (const action of turn.actions ?? []) {
          const city = action.items?.find(
            (i) =>
              i.player === seat &&
              i.origin?.Ability === 'Setup' &&
              i.Structure?.structure === 'CityCenter' &&
              i.Structure.balance === 'Gain',
          )?.Structure?.position;
          if (city) return city;
          for (const line of action.log ?? []) {
            const match = line.match(/^(.+?): Setup: .*\bGain city ([A-Z]+\d+)\b/);
            if (match && names.includes(match[1])) return match[2];
          }
        }
  }
  return (
    game.board_history?.frames[0]?.players.find((p) => p.id === seat)?.cities?.[0]?.position ??
    player.cities?.[0]?.position
  );
}

/** Fit the rotated board, with the home side pointing down on screen. */
export function strategyFrame(positions: string[], home: string | undefined, aspect: number) {
  const points = positions.map(positionXY);
  const bounds = (values: number[]) => [Math.min(...values), Math.max(...values)];
  const [left, right] = bounds(points.map(([x]) => x));
  const [top, bottom] = bounds(points.map(([, z]) => z));
  const [hx, hz] = home ? positionXY(home) : [(left + right) / 2, bottom];
  const angle = Math.atan2(hx - (left + right) / 2, hz - (top + bottom) / 2);
  const sin = Math.sin(angle),
    cos = Math.cos(angle);
  const [minX, maxX] = bounds(points.map(([x, z]) => x * cos - z * sin));
  const [minZ, maxZ] = bounds(points.map(([x, z]) => x * sin + z * cos));
  const x = (minX + maxX) / 2,
    z = (minZ + maxZ) / 2;
  return {
    angle,
    center: [x * cos + z * sin, z * cos - x * sin] as [number, number],
    distance: Math.max((maxX - minX + 2.8) / aspect, maxZ - minZ + 4) / (2 * Math.tan(Math.PI / 10)),
  };
}
