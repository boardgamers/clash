import * as THREE from 'three';

/** Fit every board corner in a fixed, oblique public preview. */
export function thumbnailCamera(bounds: THREE.Box3, width: number, height: number, parts = [bounds]) {
  const camera = new THREE.PerspectiveCamera(36, width / height, 0.1, 500);
  const center = bounds.getCenter(new THREE.Vector3());
  const direction = new THREE.Vector3(12.5, 22, 17).normalize();
  camera.position.copy(center).add(direction);
  camera.lookAt(center);
  camera.updateMatrixWorld(true);
  const inverse = camera.quaternion.clone().invert();
  const tanY = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2));
  const tanX = tanY * camera.aspect;
  let distance = 1;
  for (const part of parts)
    for (const x of [part.min.x, part.max.x])
      for (const y of [part.min.y, part.max.y])
        for (const z of [part.min.z, part.max.z]) {
          const point = new THREE.Vector3(x, y, z).sub(center).applyQuaternion(inverse);
          distance = Math.max(
            distance,
            point.z + Math.max(Math.abs(point.x) / tanX, Math.abs(point.y) / tanY) * 1.08,
          );
        }
  camera.position.copy(center).addScaledVector(direction, distance);
  camera.updateMatrixWorld(true);
  return camera;
}
