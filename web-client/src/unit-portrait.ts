import * as THREE from 'three';
import { PieceModels } from './piece-models';
import type { UnitView } from './types';

// All portraits share one renderer; a large army never creates a context per unit.
let renderer: THREE.WebGLRenderer | undefined;
const portraits = new Map<string, string>();
export function unitPortrait(type: UnitView['type'], color: string, pirate = false): string {
  const key = JSON.stringify([type, color, pirate]);
  const cached = portraits.get(key);
  if (cached) return cached;
  renderer ??= new THREE.WebGLRenderer({ alpha: true, antialias: true });
  renderer.setSize(180, 160, false);
  renderer.setClearColor(0x000000, 0);
  const materials: THREE.Material[] = [];
  const geometries: THREE.BufferGeometry[] = [];
  const models = new PieceModels(
    (color) => {
      const material = new THREE.MeshStandardMaterial({ color, roughness: 1 });
      materials.push(material);
      return material;
    },
    (geometry, material) => {
      geometries.push(geometry);
      return new THREE.Mesh(geometry, material);
    },
  );
  try {
    const model = models.unit(type, color, pirate);
    const scene = new THREE.Scene();
    scene.add(model, new THREE.HemisphereLight(0xffffff, 0x76887f, 2.6));
    const light = new THREE.DirectionalLight(0xffffff, 3);
    light.position.set(-3, 5, 4);
    scene.add(light);
    const box = new THREE.Box3().setFromObject(model);
    const center = box.getCenter(new THREE.Vector3());
    const radius = box.getBoundingSphere(new THREE.Sphere()).radius;
    const size = Math.max(0.5, radius * 1.08);
    const camera = new THREE.OrthographicCamera(-size * 1.125, size * 1.125, size, -size, 0.1, 20);
    camera.position.copy(center).add(new THREE.Vector3(2.6, 1.9, 3.4));
    camera.lookAt(center);
    renderer.render(scene, camera);
    const url = renderer.domElement.toDataURL('image/png');
    if (portraits.size >= 128) portraits.delete(portraits.keys().next().value!);
    portraits.set(key, url);
    return url;
  } finally {
    geometries.forEach((g) => g.dispose());
    materials.forEach((m) => m.dispose());
  }
}
