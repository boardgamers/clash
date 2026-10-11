import * as THREE from 'three';
import { positionXY } from './model.ts';

/** Public trade pairings, not movement paths or submitted actions. */
export class TradeOverlay {
  readonly group = new THREE.Group();
  private signature = '';
  update(routes: { from: string; to: string }[], color: string) {
    const signature = JSON.stringify([routes, color]);
    if (signature === this.signature) return;
    this.dispose();
    this.signature = signature;
    const material = new THREE.MeshBasicMaterial({ color, depthTest: false, depthWrite: false, transparent: true, opacity: 0.9 });
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
      const [x, z] = positionXY(position);
      const ring = new THREE.Mesh(new THREE.TorusGeometry(0.83, 0.035, 5, 6), material);
      ring.rotation.x = -Math.PI / 2; ring.position.set(x, 0.65, z); ring.renderOrder = 5;
      this.group.add(ring);
    }
    if (!routes.length) material.dispose();
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
