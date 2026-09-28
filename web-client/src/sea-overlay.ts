import * as THREE from 'three';
import { positionXY } from './model';
import { seaArea, seaConnections, seaEdgePoint } from './sea-routes';
import type { Terrain } from './types';

/** Read-only geography overlay. It never creates or submits game actions. */
export class SeaOverlay {
  readonly group = new THREE.Group();
  private signature = '';
  private water = new Set<string>();
  private connections: [string, string][] = [];
  private focus: string | null = null;
  private shortcuts: [string, string][] = [];
  private visuals: {
    positions: string[];
    material: THREE.MeshBasicMaterial;
    base: number;
    navigation: boolean;
  }[] = [];

  hasWater(position: string | null): position is string {
    return !!position && this.water.has(position);
  }
  update(tiles: [string, Terrain][], paths: string[][], enabled: boolean) {
    this.group.visible = enabled;
    if (!enabled) return;
    const signature = JSON.stringify([tiles, paths]);
    if (signature === this.signature) return;
    this.dispose();
    this.signature = signature;
    this.water = new Set(tiles.filter(([, terrain]) => terrain === 'Water').map(([p]) => p));
    this.connections = seaConnections(tiles);
    const point = (position: string) => {
      const [x, z] = positionXY(position);
      return new THREE.Vector3(x, 0.52, z);
    };
    for (const position of this.water) {
      const material = this.material([position], '#e5faf2', 0.85);
      const ring = new THREE.Mesh(new THREE.TorusGeometry(0.93, 0.026, 5, 6), material);
      ring.rotation.x = -Math.PI / 2;
      ring.position.copy(point(position));
      this.group.add(ring);
    }
    for (const [from, to] of this.connections) {
      const curve = new THREE.LineCurve3(point(from), point(to));
      const material = this.material([from, to], '#e5faf2', 0.8);
      this.group.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 1, 0.032, 5, false), material));
    }
    const seen = new Set<string>();
    for (const path of paths) {
      const from = path[0],
        to = path[path.length - 1];
      if (!from || !to || path.length < 3 || point(from).distanceTo(point(to)) < 1.74) continue;
      const key = [path.join('/'), [...path].reverse().join('/')].sort()[0];
      if (seen.has(key)) continue;
      seen.add(key);
      this.shortcuts.push([from, to]);
      const outer = path.map((p) => {
        const [x, z] = seaEdgePoint(p, tiles);
        return new THREE.Vector3(x, 0.52, z);
      });
      const curve = new THREE.CatmullRomCurve3([point(from), ...outer, point(to)], false, 'centripetal');
      const material = this.material([from, to], '#725226', 0.82, true);
      const steps = Math.max(12, Math.ceil(curve.getLength() / 0.22));
      for (let i = 0; i < steps; i += 2) {
        const segment = new THREE.LineCurve3(
          curve.getPointAt(i / steps),
          curve.getPointAt(Math.min(1, (i + 1) / steps)),
        );
        this.group.add(new THREE.Mesh(new THREE.TubeGeometry(segment, 1, 0.033, 5, false), material));
      }
      if (!this.water.has(to)) {
        const ring = new THREE.Mesh(new THREE.TorusGeometry(0.75, 0.035, 5, 6), material);
        ring.rotation.x = -Math.PI / 2;
        ring.position.copy(point(to));
        this.group.add(ring);
      }
    }
    this.group.traverse((o) => {
      o.renderOrder = 3;
    });
    this.setFocus(this.focus);
  }
  private material(positions: string[], color: string, base: number, navigation = false) {
    const material = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: base,
      depthTest: false,
      depthWrite: false,
      toneMapped: false,
    });
    this.visuals.push({ positions, material, base, navigation });
    return material;
  }
  setFocus(position: string | null) {
    this.focus = position;
    const area = seaArea(position, this.connections);
    const reachable = new Set(area);
    for (const [from, to] of this.shortcuts) {
      const next = area.has(from) ? to : area.has(to) ? from : null;
      if (next && this.water.has(next)) for (const p of seaArea(next, this.connections)) reachable.add(p);
    }
    for (const { positions, material, base, navigation } of this.visuals) {
      const relevant = navigation ? area : reachable;
      material.opacity = !position || positions.some((p) => relevant.has(p)) ? base : 0.12;
    }
  }
  dispose() {
    this.group.traverse((o) => {
      if (o instanceof THREE.Mesh) o.geometry.dispose();
    });
    this.visuals.forEach((v) => v.material.dispose());
    this.visuals = [];
    this.shortcuts = [];
    this.group.clear();
    this.signature = '';
  }
}
