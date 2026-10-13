import * as THREE from 'three';
import { positionXY } from './model.ts';
import type { View } from './types';

/** Omit shared edges inside a region; keep its perimeter. */
export function regionEdges(tiles: string[]): [number[], number[]][] {
  const edges = new Map<string, [number[], number[]]>();
  for (const tile of tiles) {
    const [x, z] = positionXY(tile);
    const corners = Array.from({length:6}, (_, i) => [x + Math.cos(i*Math.PI/3), z + Math.sin(i*Math.PI/3)]);
    for (let i=0;i<6;i++) {
      const edge: [number[], number[]] = [corners[i], corners[(i+1)%6]];
      const key = edge.map(p=>p.map(v=>v.toFixed(4)).join(',')).sort().join('/');
      if (edges.has(key)) edges.delete(key); else edges.set(key, edge);
    }
  }
  return [...edges.values()];
}
export class RegionOverlay {
  readonly group = new THREE.Group();
  private signature = '';
  update(regions: NonNullable<View['regions']>, player: number) {
    const signature = JSON.stringify([regions, player]);
    if (signature === this.signature) return;
    this.dispose(); this.signature = signature;
    for (const region of regions) {
      const material = new THREE.MeshBasicMaterial({color:region.homePlayer === player ? '#ffd16c' : '#f4f7ff', depthTest:false, depthWrite:false, toneMapped:false});
      for (const [a,b] of regionEdges(region.tiles)) {
        const curve = new THREE.LineCurve3(new THREE.Vector3(a[0],.37,a[1]), new THREE.Vector3(b[0],.37,b[1]));
        const mesh = new THREE.Mesh(new THREE.TubeGeometry(curve,1,.035,4,false), material);
        mesh.renderOrder = region.homePlayer === player ? 6 : 5; this.group.add(mesh);
      }
    }
  }
  dispose() {
    const materials = new Set<THREE.Material>();
    this.group.traverse(object => { if (object instanceof THREE.Mesh) { object.geometry.dispose(); materials.add(object.material); } });
    materials.forEach(material=>material.dispose()); this.group.clear(); this.signature='';
  }
}
