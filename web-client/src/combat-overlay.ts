import * as THREE from 'three';
import { positionXY } from './model.ts';
import type { ActiveCombat } from './active-combat.ts';

/** Connect the still-unresolved attackers to the tile where combat is happening. */
export class CombatOverlay {
  readonly group = new THREE.Group();
  readonly position = new THREE.Vector3();
  private signature = '';
  private animation = 0;
  private finish: (() => void) | null = null;
  private material = new THREE.MeshBasicMaterial({
    color: '#ffdb8a',
    depthTest: false,
    depthWrite: false,
    toneMapped: false,
  });

  update(combat: ActiveCombat | null) {
    const signature = combat ? `${combat.attacker.position}/${combat.defender.position}` : '';
    if (signature === this.signature) return;
    this.clear();
    this.signature = signature;
    if (!combat) return;
    const [x, z] = positionXY(combat.defender.position);
    this.position.set(x, 0.58, z);
    const ring = new THREE.Mesh(new THREE.TorusGeometry(1.01, 0.035, 6, 6), this.material);
    ring.rotation.x = -Math.PI / 2;
    ring.position.copy(this.position);
    this.group.add(ring);
    const [fromX, fromZ] = positionXY(combat.attacker.position);
    const from = new THREE.Vector3(fromX, 0.58, fromZ);
    const direction = this.position.clone().sub(from);
    const distance = direction.length();
    if (distance > 0.8) {
      direction.normalize();
      const end = this.position.clone().addScaledVector(direction, -0.45);
      from.addScaledVector(direction, 0.3);
      const shaftEnd = end.clone().addScaledVector(direction, -0.16);
      this.group.add(
        new THREE.Mesh(
          new THREE.TubeGeometry(new THREE.LineCurve3(from, shaftEnd), 1, 0.045, 6, false),
          this.material,
        ),
      );
      const head = new THREE.Mesh(new THREE.ConeGeometry(0.16, 0.3, 3), this.material);
      head.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction);
      head.position.copy(shaftEnd);
      this.group.add(head);
    }
    this.group.traverse((object) => {
      object.renderOrder = 8;
    });
  }
  /** A single expanding impact wave, with no camera shake or flashing. */
  pulse(invalidate: () => void) {
    this.finish?.();
    if (!this.signature) return;
    const material = new THREE.MeshBasicMaterial({
      color: '#ffd98a',
      transparent: true,
      opacity: 0.8,
      depthTest: false,
      depthWrite: false,
      toneMapped: false,
    });
    const wave = new THREE.Mesh(new THREE.RingGeometry(0.45, 0.52, 48), material);
    wave.rotation.x = -Math.PI / 2;
    wave.position.copy(this.position).add(new THREE.Vector3(0, 0.08, 0));
    wave.renderOrder = 9;
    this.group.add(wave);
    const start = performance.now();
    this.finish = () => {
      cancelAnimationFrame(this.animation);
      this.group.remove(wave);
      wave.geometry.dispose();
      material.dispose();
      this.finish = null;
      invalidate();
    };
    const tick = () => {
      const t = Math.min(1, (performance.now() - start) / 1350);
      wave.scale.setScalar(0.3 + t * 3.4);
      material.opacity = (1 - t) * 0.8;
      invalidate();
      if (t < 1) this.animation = requestAnimationFrame(tick);
      else this.finish?.();
    };
    tick();
  }
  private clear() {
    this.finish?.();
    this.group.traverse((object) => {
      if (object instanceof THREE.Mesh) object.geometry.dispose();
    });
    this.group.clear();
  }
  stopAnimation() {
    this.finish?.();
  }
  dispose() {
    this.clear();
    this.material.dispose();
  }
}
