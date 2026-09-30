import * as THREE from 'three';
import { positionXY } from './model.ts';
import type { ActiveCombat } from './active-combat.ts';

/** Connect the still-unresolved attackers to the tile where combat is happening. */
export class CombatOverlay {
  readonly group = new THREE.Group();
  readonly position = new THREE.Vector3();
  private signature = '';
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
  private clear() {
    this.group.traverse((object) => {
      if (object instanceof THREE.Mesh) object.geometry.dispose();
    });
    this.group.clear();
  }
  dispose() {
    this.clear();
    this.material.dispose();
  }
}
