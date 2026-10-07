import type { BuildingKind } from './piece-models.ts';

export interface PiecePlacement {
  x: number;
  z: number;
  scale: number;
}

export interface CityLayout {
  settlement: PiecePlacement;
  /** Ordinary buildings, in `buildingOrder` order. */
  buildings: PiecePlacement[];
  wonders: PiecePlacement[];
  /** Offset of the flag pole, flag, crown and colorblind badge from `ownershipMarkers`. */
  ownership: { x: number; z: number };
}

/** Tallest buildings first, so they take the rear slots. */
export const buildingOrder: BuildingKind[] = [
  'obelisk',
  'observatory',
  'fortress',
  'temple',
  'port',
  'academy',
  'market',
];

/** Flag pole, flag and capital crown, relative to the city model (before the layout's `ownership` offset). */
export const ownershipMarkers = {
  pole: { radiusTop: 0.018, radiusBottom: 0.024, height: 1.06, x: 0.12, y: 0.54, z: -0.12 },
  flag: { width: 0.45, height: 0.3, depth: 0.025, x: 0.345, y: 0.93, z: -0.12 },
  crown: { radius: 0.16, height: 0.07, y: 1.12, pointY: 1.22, pointSpacing: 0.11 },
  badge: { x: 0.27, y: 1.27 },
};

/** The city model sits on top of its land tile, slightly turned to show two building faces. */
export const cityPose = { y: 0.315, rotation: 0.25 };

/** Water-level wharf placed in the Port's sea hex, with a gangway up to the city. */
export const portDock = {
  scale: 0.5,
  y: 0.01,
  gangway: { width: 0.16, height: 0.04, depth: 0.36, y: 0.19, z: -0.31, tilt: 0.53 },
};

/**
 * Positions of the settlement, ordinary buildings and wonders inside one city hex,
 * in the city model's local frame. Kept separate from the board so the overlap test
 * can check every civilization and city size without a renderer.
 */
export function cityLayout(buildings: number, wonders: number): CityLayout {
  if (wonders) return landmarkLayout(buildings, wonders);
  // The settlement and its flag move together; the tallest buildings take the rear slots.
  // Rows are spaced by each piece's widest roof or temple so no two pieces share ground.
  const [settlement, scale, slots]: [PiecePlacement, number, number[][]] =
    buildings === 0
      ? [{ x: 0, z: -0.09, scale: 0.88 }, 0, []]
      : buildings === 1
        ? [{ x: -0.2, z: -0.09, scale: 0.57 }, 0.62, [[0.4, 0.18]]]
        : buildings === 2
          ? [
              { x: 0, z: -0.06, scale: 0.53 },
              0.56,
              [
                [-0.55, 0.1],
                [0.55, 0.1],
              ],
            ]
          : buildings === 3
            ? [
                { x: 0, z: -0.1, scale: 0.53 },
                0.55,
                [
                  [0, -0.6],
                  [-0.42, 0.36],
                  [0.42, 0.36],
                ],
              ]
            : [
                { x: 0, z: -0.02, scale: 0.5 },
                0.5,
                [
                  [-0.28, -0.5],
                  [0.28, -0.5],
                  [-0.28, 0.47],
                  [0.28, 0.47],
                ],
              ];
  return {
    settlement,
    buildings: slots.map(([x, z]) => ({ x, z, scale })),
    wonders: [],
    // A lone annex leaves the flag on the open side of the shifted settlement.
    ownership: { x: buildings === 1 ? 0 : settlement.x, z: settlement.z + 0.09 },
  };
}

/** Landmarks fill the rear of the hex; the settlement, its flag and ordinary buildings stay in front. */
function landmarkLayout(buildings: number, wonders: number): CityLayout {
  const pieces = buildings + 1,
    scale = pieces > 3 ? 0.36 : 0.46,
    spacing = pieces > 3 ? 0.43 : 0.47;
  const ordinary = Array.from({ length: pieces }, (_, index) => {
    const row = Math.floor(index / 3),
      rowCount = Math.min(3, pieces - row * 3);
    return { x: ((index % 3) - (rowCount - 1) / 2) * spacing, z: row ? 0.57 : 0.2, scale };
  });
  const columns = Math.min(3, wonders),
    wonderScale =
      wonders === 1 ? 0.8 : wonders === 2 ? 0.57 : wonders === 3 ? 0.4 : wonders <= 6 ? 0.28 : 0.24;
  const landmarks = Array.from({ length: wonders }, (_, index) => ({
    x: ((index % columns) - (columns - 1) / 2) * wonderScale * 1.08,
    z:
      wonders === 1 ? -0.36 : wonders <= 3 ? -0.33 : -0.58 + Math.floor(index / columns) * wonderScale * 0.86,
    scale: wonderScale,
  }));
  const [settlement] = ordinary;
  return {
    settlement,
    buildings: ordinary.slice(1),
    wonders: landmarks,
    ownership: { x: settlement.x, z: settlement.z + 0.09 },
  };
}
