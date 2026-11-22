import * as THREE from 'three';
import { positionXY } from './model.ts';
import { tileSurfaceHeight } from './terrain-display.ts';
import type { Terrain } from './types.ts';

/** Public trade pairings, not movement paths or submitted actions. */
export class TradeOverlay {
  readonly group = new THREE.Group();
  private signature = '';
  update(routes: { from: string; to: string }[], color: string, raidedPositions: string[] = [], tiles: [string, Terrain][] = []) {
    const surfaces = new Map(tiles.map(([position, terrain]) => [position, tileSurfaceHeight(terrain)]));
    const contourHeight = (position: string) => (surfaces.get(position) ?? tileSurfaceHeight('Fertile')) + 0.04;
    const signature = JSON.stringify([routes, color, raidedPositions, tiles]);
    if (signature === this.signature) return;
    this.dispose();
    this.signature = signature;
    const material = new THREE.MeshBasicMaterial({ color, depthTest: false, depthWrite: false, transparent: true, opacity: 0.9 });
    const raided = new Set(raidedPositions);
    const endpoints = new Set<string>();
    for (const { from, to } of routes) {
      endpoints.add(from); endpoints.add(to);
      const [x1, z1] = positionXY(from), [x2, z2] = positionXY(to);
      const start = new THREE.Vector3(x1, 0.65, z1), end = new THREE.Vector3(x2, 0.65, z2);
      const midpoint = start.clone().lerp(end, 0.5); midpoint.y += 0.65;
      const curve = new THREE.QuadraticBezierCurve3(start, midpoint, end);
      const line = new THREE.Mesh(new THREE.TubeGeometry(curve, 24, 0.045, 6, false), material);
      line.renderOrder = 5; this.group.add(line);
    }
    for (const position of endpoints) {
      if (raided.has(position)) continue;
      const [x, z] = positionXY(position);
      const ring = new THREE.Mesh(new THREE.TorusGeometry(0.83, 0.035, 5, 6), material);
      ring.rotation.x = -Math.PI / 2; ring.position.set(x, contourHeight(position), z); ring.renderOrder = 5;
      this.group.add(ring);
    }
    if (!routes.length) material.dispose();
    // Keep valid route lines, including ships sharing a raided Settler hex.
    // Only the endpoint contour is replaced with orange, avoiding stacked rings.
    if (raided.size) {
      // Depth-test against terrain and pieces instead of painting over their silhouettes.
      const warning = new THREE.MeshBasicMaterial({ color: '#ed8a23', depthTest: true, depthWrite: false, toneMapped: false });
      for (const position of raided) {
        const [x, z] = positionXY(position);
        const ring = new THREE.Mesh(new THREE.TorusGeometry(0.94, 0.025, 6, 6), warning);
        ring.rotation.x = -Math.PI / 2;
        // The tube sits just above the slab, not at the old fixed unit-height 0.67.
        ring.position.set(x, contourHeight(position), z);
        ring.renderOrder = 6;
        this.group.add(ring);
      }
    }
  }

  dispose() {
    const materials = new Set<THREE.Material>();
    this.group.traverse(object => {
      if (object instanceof THREE.Mesh) {
        object.geometry.dispose();
        for (const material of Array.isArray(object.material) ? object.material : [object.material]) materials.add(material);
      }
    });
    for (const material of materials) material.dispose();
    this.group.clear(); this.signature = '';
  }
}
