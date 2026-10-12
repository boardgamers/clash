import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { positionXY } from './model.ts';

/** Dashed, two-tone borders remain distinct from legal-move and selection rings. */
export class PirateBlockadeOverlay {
  readonly group = new THREE.Group();
  private signature = '';

  update(positions: string[]) {
    const signature = JSON.stringify(positions);
    if (signature === this.signature) return;
    this.dispose();
    this.signature = signature;
    if (!positions.length) return;
    for (const [color, width, height] of [
      ['#ffebc8', 0.075, 0.39],
      ['#b3332b', 0.038, 0.405],
    ] as const) {
      const segments: THREE.BufferGeometry[] = [];
      for (const position of positions) {
        const [x, z] = positionXY(position);
        for (let side = 0; side < 6; side++) {
          const a = (side * Math.PI) / 3,
            b = ((side + 1) * Math.PI) / 3;
          const start = new THREE.Vector3(x + 0.89 * Math.cos(a), height, z + 0.89 * Math.sin(a));
          const end = new THREE.Vector3(x + 0.89 * Math.cos(b), height, z + 0.89 * Math.sin(b));
          for (const offset of [0.08, 0.58]) {
            const from = start.clone().lerp(end, offset),
              to = start.clone().lerp(end, offset + 0.34);
            const geometry = new THREE.BoxGeometry(from.distanceTo(to), 0.012, width);
            geometry.rotateY(-Math.atan2(to.z - from.z, to.x - from.x));
            geometry.translate((from.x + to.x) / 2, height, (from.z + to.z) / 2);
            segments.push(geometry);
          }
        }
      }
      const geometry = mergeGeometries(segments);
      segments.forEach((segment) => segment.dispose());
      const mesh = new THREE.Mesh(geometry, new THREE.MeshBasicMaterial({ color, toneMapped: false }));
      this.group.add(mesh);
    }
  }

  dispose() {
    for (const child of this.group.children) {
      const mesh = child as THREE.Mesh<THREE.BufferGeometry, THREE.Material>;
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.group.clear();
    this.signature = '';
  }
}
