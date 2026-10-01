import * as THREE from 'three';
import { positionXY } from './model.ts';

/** One continuous boundary identifies the whole revealed region, not just the arrival hex. */
export class ExplorationOverlay {
  readonly group = new THREE.Group();
  readonly position = new THREE.Vector3();
  private signature = '';
  private border = new THREE.MeshBasicMaterial({
    color: '#ffd16b',
    depthTest: false,
    depthWrite: false,
    toneMapped: false,
  });
  private fill = new THREE.MeshBasicMaterial({
    color: '#ffc450',
    transparent: true,
    opacity: 0.13,
    depthWrite: false,
    side: THREE.DoubleSide,
  });
  update(positions: string[]) {
    const signature = positions.slice().sort().join('/');
    if (signature === this.signature) return;
    this.clear();
    this.signature = signature;
    if (!positions.length) return;
    const points = positions.map(positionXY);
    const edges = new Map<string, { a: THREE.Vector3; b: THREE.Vector3; count: number }>();
    const key = (point: THREE.Vector3) => `${Number(point.x.toFixed(4))},${Number(point.z.toFixed(4))}`;
    for (const [x, z] of points) {
      const fill = new THREE.Mesh(new THREE.CircleGeometry(0.97, 6), this.fill);
      fill.rotation.x = -Math.PI / 2;
      fill.position.set(x, 0.34, z);
      this.group.add(fill);
      const vertices = Array.from(
        { length: 6 },
        (_, i) => new THREE.Vector3(x + Math.cos((i * Math.PI) / 3), 0.43, z + Math.sin((i * Math.PI) / 3)),
      );
      for (let i = 0; i < 6; i++) {
        const a = vertices[i],
          b = vertices[(i + 1) % 6],
          id = [key(a), key(b)].sort().join('/');
        const edge = edges.get(id);
        if (edge) edge.count++;
        else edges.set(id, { a, b, count: 1 });
      }
    }
    for (const { a, b, count } of edges.values())
      if (count === 1) {
        const line = new THREE.Mesh(
          new THREE.TubeGeometry(new THREE.LineCurve3(a, b), 1, 0.045, 6, false),
          this.border,
        );
        line.renderOrder = 9;
        this.group.add(line);
      }
    this.position.set(
      points.reduce((n, p) => n + p[0], 0) / points.length,
      0.5,
      Math.max(...points.map((p) => p[1])) + 0.8,
    );
  }
  private clear() {
    this.group.traverse((o) => {
      if (o instanceof THREE.Mesh) o.geometry.dispose();
    });
    this.group.clear();
  }
  dispose() {
    this.clear();
    this.border.dispose();
    this.fill.dispose();
  }
}
