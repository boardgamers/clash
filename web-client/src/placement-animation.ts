import * as THREE from 'three';

export const PLACEMENT_DURATION = 1100;

/** Bring newly placed pieces into view with a brief glow at their location. */
export class PlacementAnimation {
  readonly group = new THREE.Group();
  private frame = 0;
  private finish: (() => void) | null = null;

  play(pieces: { model: THREE.Group; kind: 'building' | 'unit' }[], invalidate: () => void) {
    this.stop();
    if (!pieces.length) return;
    const models = pieces.map(({ model, kind }) => ({
      model,
      kind,
      position: model.position.clone(),
      scale: model.scale.clone(),
    }));
    const waves = pieces.map(({ model }) => {
      const material = new THREE.MeshBasicMaterial({
        color: '#ffda8b',
        transparent: true,
        opacity: 0,
        depthTest: false,
        depthWrite: false,
        toneMapped: false,
        side: THREE.DoubleSide,
      });
      const wave = new THREE.Mesh(new THREE.RingGeometry(0.32, 0.37, 48), material);
      model.getWorldPosition(wave.position);
      wave.position.y += 0.035;
      wave.rotation.x = -Math.PI / 2;
      wave.renderOrder = 7;
      this.group.add(wave);
      return wave;
    });
    this.finish = () => {
      cancelAnimationFrame(this.frame);
      for (const { model, position, scale } of models) {
        model.position.copy(position);
        model.scale.copy(scale);
      }
      for (const wave of waves) {
        this.group.remove(wave);
        wave.geometry.dispose();
        wave.material.dispose();
      }
      this.finish = null;
      invalidate();
    };
    const start = performance.now();
    const tick = () => {
      const elapsed = performance.now() - start;
      const t = Math.min(1, elapsed / PLACEMENT_DURATION);
      const rise = Math.min(1, elapsed / 850);
      const eased = 1 - (1 - rise) ** 3;
      for (const { model, kind, position, scale } of models) {
        model.position.copy(position);
        if (kind === 'unit') {
          model.position.y += 0.4 * (1 - eased);
          model.scale.copy(scale).multiplyScalar(0.2 + 0.8 * eased + 0.06 * Math.sin(Math.PI * rise));
          continue;
        }
        model.position.y -= 0.08 * (1 - eased);
        model.scale.set(
          scale.x * (0.8 + 0.2 * eased),
          scale.y * (0.04 + 0.96 * eased),
          scale.z * (0.8 + 0.2 * eased),
        );
      }
      for (const wave of waves) {
        wave.scale.setScalar(0.65 + 1.75 * t);
        wave.material.opacity = 0.6 * (1 - t);
      }
      invalidate();
      if (t < 1) this.frame = requestAnimationFrame(tick);
      else this.stop();
    };
    tick();
  }

  stop() {
    this.finish?.();
  }
}
