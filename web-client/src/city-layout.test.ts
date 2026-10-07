import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { PieceModels, type BuildingKind } from './piece-models.ts';
import { civilizationPieceStyles } from './piece-styles.ts';
import {
  buildingOrder,
  cityLayout,
  cityPose,
  ownershipMarkers,
  portDock,
  type PiecePlacement,
} from './city-layout.ts';
import { portPlacement } from './port-layout.ts';
import { positionXY } from './model.ts';

// Visible gaps smaller than this are fine; anything deeper is a part sinking into another.
const TOLERANCE = 0.004;
const civilizations: (string | undefined)[] = [...Object.keys(civilizationPieceStyles), undefined];
const wonderKinds = [
  'Pyramids',
  'GreatGardens',
  'Colosseum',
  'GreatLibrary',
  'GreatLighthouse',
  'GreatMausoleum',
  'GreatStatue',
  'GreatWall',
];
const material = new THREE.MeshBasicMaterial();
const models = new PieceModels(
  () => material,
  (geometry, mat) => new THREE.Mesh(geometry, mat),
);

/** Tight per-mesh bounds of an object, in its parent's frame at identity transform. */
function meshBoxes(object: THREE.Object3D) {
  object.updateMatrixWorld(true);
  const boxes: THREE.Box3[] = [];
  object.traverse((child) => {
    if (child instanceof THREE.Mesh) boxes.push(new THREE.Box3().setFromObject(child, true));
  });
  return boxes;
}
const depth = (p: THREE.Box3, q: THREE.Box3) =>
  Math.min(
    Math.min(p.max.x, q.max.x) - Math.max(p.min.x, q.min.x),
    Math.min(p.max.y, q.max.y) - Math.max(p.min.y, q.min.y),
    Math.min(p.max.z, q.max.z) - Math.max(p.min.z, q.min.z),
  );
/** A piece's parts, with their overall bounds to skip distant pairs quickly. */
type Parts = { boxes: THREE.Box3[]; bounds: THREE.Box3 };
const parts = (boxes: THREE.Box3[]): Parts => ({
  boxes,
  bounds: boxes.reduce((all, box) => all.union(box), new THREE.Box3()),
});
const placedCache = new Map<string, Parts>();
function placed(key: string, boxes: THREE.Box3[], { x, z, scale }: PiecePlacement) {
  const id = `${key}@${x},${z},${scale}`;
  if (!placedCache.has(id))
    placedCache.set(
      id,
      parts(
        boxes.map((box) => {
          const result = box.clone();
          result.min.multiplyScalar(scale).add(new THREE.Vector3(x, 0, z));
          result.max.multiplyScalar(scale).add(new THREE.Vector3(x, 0, z));
          return result;
        }),
      ),
    );
  return placedCache.get(id)!;
}
/** Whether any part sinks deeper than the tolerance into a part of the other piece. */
function intersects(a: Parts, b: Parts) {
  if (depth(a.bounds, b.bounds) <= TOLERANCE) return false;
  return a.boxes.some((p) => depth(p, b.bounds) > TOLERANCE && b.boxes.some((q) => depth(p, q) > TOLERANCE));
}

/** Flag pole, flag and capital crown share the city center with the settlement. */
const ownershipCache = new Map<string, THREE.Box3[]>();
function ownershipBoxes({ x: dx, z: dz }: { x: number; z: number }) {
  const key = `${dx},${dz}`;
  if (ownershipCache.has(key)) return ownershipCache.get(key)!;
  const g = new THREE.Group(),
    { pole, flag, crown } = ownershipMarkers;
  const add = (geometry: THREE.BufferGeometry, x: number, y: number, z: number) => {
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(x + dx, y, z + dz);
    g.add(mesh);
  };
  add(new THREE.CylinderGeometry(pole.radiusTop, pole.radiusBottom, pole.height, 6), pole.x, pole.y, pole.z);
  add(new THREE.BoxGeometry(flag.width, flag.height, flag.depth), flag.x, flag.y, flag.z);
  add(new THREE.CylinderGeometry(crown.radius, crown.radius, crown.height, 12), pole.x, crown.y, pole.z);
  for (let i = 0; i < 3; i++)
    add(new THREE.ConeGeometry(0.055, 0.15, 4), pole.x + (i - 1) * crown.pointSpacing, crown.pointY, pole.z);
  ownershipCache.set(key, meshBoxes(g));
  return ownershipCache.get(key)!;
}

function combinations<T>(items: T[], size: number): T[][] {
  if (!size) return [[]];
  return items.flatMap((item, i) =>
    combinations(items.slice(i + 1), size - 1).map((rest) => [item, ...rest]),
  );
}

const settlementBoxes = new Map(civilizations.map((c) => [c, meshBoxes(models.settlement('#888', c))]));
const buildingBoxes = new Map(
  civilizations.map((c) => [
    c,
    new Map(buildingOrder.map((kind) => [kind, meshBoxes(models.building(kind, '#888', c))])),
  ]),
);
const wonderBoxes = new Map(wonderKinds.map((kind) => [kind, meshBoxes(models.wonder(kind, '#888'))]));

/**
 * Every civilization (and the neutral fallback), every building set a city can hold
 * (at most four pieces besides the settlement) and every wonder in every landmark slot.
 */
function* cities(maxWonders = wonderKinds.length) {
  for (const civilization of civilizations)
    for (let size = 0; size <= 4; size++)
      for (const kinds of combinations(buildingOrder, size))
        for (let wonders = 0; wonders <= maxWonders; wonders++)
          for (let offset = 0; offset < (wonders ? wonderKinds.length : 1); offset++)
            yield {
              civilization,
              kinds,
              wonders: Array.from(
                { length: wonders },
                (_, i) => wonderKinds[(offset + i) % wonderKinds.length],
              ),
            };
}

const centers = new Map<string, Parts>();
function cityPieces(civilization: string | undefined, kinds: BuildingKind[], wonders: string[]) {
  const layout = cityLayout(kinds.length, wonders.length);
  const settlement = placed(`${civilization}`, settlementBoxes.get(civilization)!, layout.settlement);
  const key = `${civilization} ${kinds.length} ${wonders.length}`;
  if (!centers.has(key)) centers.set(key, parts([...settlement.boxes, ...ownershipBoxes(layout.ownership)]));
  const center = centers.get(key)!;
  return [
    { name: 'city center', parts: center },
    ...kinds.map((kind, i) => ({
      name: kind,
      parts: placed(
        `${civilization} ${kind}`,
        buildingBoxes.get(civilization)!.get(kind)!,
        layout.buildings[i],
      ),
    })),
    ...wonders.map((kind, i) => ({
      name: kind,
      parts: placed(kind, wonderBoxes.get(kind)!, layout.wonders[i]),
    })),
  ];
}

test('city pieces never clip into the city center or each other', () => {
  const clipping = new Set<string>();
  for (const { civilization, kinds, wonders } of cities()) {
    const pieces = cityPieces(civilization, kinds, wonders);
    for (let i = 0; i < pieces.length; i++)
      for (let j = i + 1; j < pieces.length; j++)
        if (intersects(pieces[i].parts, pieces[j].parts))
          clipping.add(
            `${civilization ?? 'Neutral'} size ${kinds.length + 1} +${wonders.length} wonders: ${pieces[i].name} × ${pieces[j].name}`,
          );
  }
  assert.deepEqual([...clipping].slice(0, 40), [], `${clipping.size} clipping combinations`);
});

test('city foundations stay inside their hex', () => {
  // Flat-topped hexes with unit circumradius; the city model is turned by cityPose.rotation.
  // Roofs and flags may overhang the edge, but nothing may stand on a neighboring tile.
  const apothem = Math.sqrt(3) / 2;
  const checked = new Set<Parts>(),
    outside = new Set<string>();
  for (const { civilization, kinds, wonders } of cities())
    for (const piece of cityPieces(civilization, kinds, wonders)) {
      if (checked.has(piece.parts)) continue;
      checked.add(piece.parts);
      const out = piece.parts.boxes.some(
        (box) =>
          box.min.y <= 0.03 &&
          [box.min.x, box.max.x].some((x) =>
            [box.min.z, box.max.z].some((z) => {
              const corner = new THREE.Vector2(x, z).rotateAround(new THREE.Vector2(), -cityPose.rotation);
              return Array.from({ length: 6 }, (_, side) => (side * Math.PI) / 3 + Math.PI / 6).some(
                (normal) => corner.x * Math.cos(normal) + corner.y * Math.sin(normal) > apothem + TOLERANCE,
              );
            }),
          ),
      );
      if (out)
        outside.add(
          `${civilization ?? 'Neutral'} size ${kinds.length + 1} +${wonders.length}: ${piece.name}`,
        );
    }
  assert.deepEqual([...outside].slice(0, 40), [], `${outside.size} pieces leave the hex`);
});

test('a docked Port never clips into its city in any direction', () => {
  const clipping = new Set<string>();
  const neighbors = ['C2', 'C4', 'B2', 'B3', 'D2', 'D3'];
  for (const civilization of civilizations)
    for (const water of neighbors) {
      const site = portPlacement('C3', water);
      assert.ok(site, water);
      // Build the dock as the board does, then express it in the city model's frame.
      const frame = new THREE.Group();
      const dock = new THREE.Group();
      const port = models.building('port', '#888', civilization);
      port.scale.setScalar(portDock.scale);
      dock.add(port);
      const plank = portDock.gangway;
      const gangway = new THREE.Mesh(new THREE.BoxGeometry(plank.width, plank.height, plank.depth), material);
      gangway.position.set(0, plank.y, plank.z);
      gangway.rotation.x = plank.tilt;
      dock.add(gangway);
      frame.add(dock);
      dock.position.set(site.x, portDock.y, site.z);
      dock.rotation.y = site.rotation;
      // Inverse of the city model's transform at C3.
      const cityModel = new THREE.Object3D();
      const [hx, hz] = positionXY('C3');
      cityModel.position.set(hx, cityPose.y, hz);
      cityModel.rotation.y = cityPose.rotation;
      cityModel.updateMatrixWorld(true);
      frame.applyMatrix4(cityModel.matrixWorld.clone().invert());
      const docked = parts(meshBoxes(frame));
      const inland = buildingOrder.filter((kind) => kind !== 'port');
      for (let size = 0; size <= 4; size++)
        for (const kinds of combinations(inland, size))
          for (const wonders of [[], ['GreatWall'], wonderKinds.slice(0, 3), wonderKinds.slice(3, 7)])
            for (const piece of cityPieces(civilization, kinds, wonders))
              if (intersects(docked, piece.parts))
                clipping.add(`${civilization ?? 'Neutral'} port toward ${water}: ${piece.name}`);
    }
  assert.deepEqual([...clipping].slice(0, 40), [], `${clipping.size} clipping docks`);
});
