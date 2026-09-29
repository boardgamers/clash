import CollectionMapBadge from './CollectionMapBadge.svelte';
import { collectionYield, collectionBonusLabel, sameCollection } from './collection-yield';
import type { MapPick } from './types';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Session, Terrain } from './types';
import { playerColor, playerSymbol } from './types';
import { positionXY } from './model';
import { MapGesture } from './map-gesture';
import { SeaOverlay } from './sea-overlay';
import { mount, unmount } from 'svelte';
import UnitMapBadge from './UnitMapBadge.svelte';

const terrainColor: Record<string, string> = {
  Forest: '#54755a',
  Fertile: '#9aa36b',
  Mountain: '#858c87',
  Barren: '#b6a27c',
  Water: '#438d95',
  Unexplored: '#526f77',
};
export class World {
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(36, 1, 0.1, 120);
  private renderer: THREE.WebGLRenderer;
  private controls: OrbitControls;
  private board = new THREE.Group();
  private rings = new THREE.Group();
  private seaOverlay = new SeaOverlay();
  private seaGuide = false;
  private seaPreviewAllowed = false;
  private seaRouteStart: string | null = null;
  private tiles = new Map<string, THREE.Mesh>();
  private pieces: THREE.Group[] = [];
  private hoverRing: THREE.Mesh;
  private hovered: string | null = null;
  private referenceRing: THREE.Mesh;
  private referenceLabel: HTMLDivElement;
  private pinnedReference: string | null = null;
  private selectable: Set<string> | null = null;
  private pending = false;
  private gesture = new MapGesture();
  private raycaster = new THREE.Raycaster();
  private resize: ResizeObserver;
  private frame = 0;
  private dirty = true;
  private lastSignature = '';
  private selectionSignature = '';
  private moveMarkerSignature = '';
  private collectionSignature = '';
  private collectionBadges = new Map<
    string,
    { node: HTMLButtonElement; component: ReturnType<typeof mount> }
  >();
  private disposed = false;
  private topDown = false;
  private boardInteraction = false;
  private materials = new Set<THREE.Material>();
  private geometries = new Set<THREE.BufferGeometry>();
  private textures = new Set<THREE.Texture>();
  private labelPositions: {
    position: string;
    at: THREE.Vector3;
    node: HTMLButtonElement;
    kind: 'city' | 'units' | 'destination' | 'collection';
    offsetY?: number;
  }[] = [];
  private unitBadges: ReturnType<typeof mount>[] = [];
  private labelHost: HTMLDivElement;
  private center = new THREE.Vector3(4.5, 0, 8);
  private material(color: string, roughness = 1) {
    const mat = new THREE.MeshStandardMaterial({ color, roughness, flatShading: true });
    this.materials.add(mat);
    return mat;
  }
  private mesh(geo: THREE.BufferGeometry, mat: THREE.Material | THREE.Material[]) {
    this.geometries.add(geo);
    const obj = new THREE.Mesh(geo, mat);
    obj.castShadow = true;
    obj.receiveShadow = true;
    return obj;
  }
  constructor(
    private host: HTMLElement,
    private pick: (position: string, source?: MapPick) => void,
    private hover: () => void = () => {},
    private dismiss: () => void = () => {},
  ) {
    this.scene.background = new THREE.Color('#b1c7c0');
    this.scene.fog = new THREE.Fog('#b1c7c0', 55, 105);
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 1.8));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.renderer.domElement.setAttribute(
      'aria-label',
      '3D civilization map. Select a city or terrain tile. Drag to orbit; scroll to zoom.',
    );
    this.renderer.domElement.setAttribute('role', 'img');
    this.renderer.domElement.tabIndex = 0;
    host.append(this.renderer.domElement);
    this.labelHost = document.createElement('div');
    this.labelHost.className = 'world-labels';
    host.append(this.labelHost);
    this.scene.add(new THREE.HemisphereLight('#fff4d8', '#386775', 2.2));
    const sun = new THREE.DirectionalLight('#ffe0a7', 3.2);
    sun.position.set(-9, 18, 8);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    sun.shadow.camera.left = -18;
    sun.shadow.camera.right = 18;
    sun.shadow.camera.top = 18;
    sun.shadow.camera.bottom = -18;
    sun.shadow.normalBias = 0.045;
    this.scene.add(sun);
    const water = this.mesh(new THREE.PlaneGeometry(150, 150), this.material('#679a9d', 0.4));
    water.rotation.x = -Math.PI / 2;
    water.position.y = -0.56;
    water.receiveShadow = true;
    this.scene.add(water);
    const glintMat = this.material('#a1c1b8');
    for (let i = 0; i < 130; i++) {
      const x = Math.sin(i * 99.31) * 22 + 4.5,
        z = Math.cos(i * 47.79) * 22 + 8;
      const glint = this.mesh(new THREE.PlaneGeometry(0.12 + (i % 5) * 0.06, 0.018), glintMat);
      glint.rotation.x = -Math.PI / 2;
      glint.position.set(x, -0.535, z);
      this.scene.add(glint);
    }
    this.scene.add(this.board, this.rings);
    const hoverMaterial = new THREE.MeshBasicMaterial({
      color: '#fff3c9',
      transparent: true,
      opacity: 0.9,
      depthWrite: false,
    });
    this.materials.add(hoverMaterial);
    this.hoverRing = this.mesh(new THREE.TorusGeometry(0.98, 0.025, 6, 6), hoverMaterial);
    this.hoverRing.rotation.x = -Math.PI / 2;
    this.hoverRing.visible = false;
    this.hoverRing.castShadow = false;
    this.scene.add(this.hoverRing);
    this.scene.add(this.seaOverlay.group);
    const referenceMaterial = new THREE.MeshBasicMaterial({ color: '#fff3bd', depthTest: false });
    this.materials.add(referenceMaterial);
    this.referenceRing = this.mesh(new THREE.TorusGeometry(1.01, 0.045, 6, 6), referenceMaterial);
    this.referenceRing.rotation.x = -Math.PI / 2;
    this.referenceRing.visible = false;
    this.referenceRing.castShadow = false;
    this.referenceRing.renderOrder = 10;
    this.scene.add(this.referenceRing);
    this.referenceLabel = document.createElement('div');
    this.referenceLabel.className = 'map-coordinate-reference';
    this.referenceLabel.hidden = true;
    this.referenceLabel.setAttribute('role', 'status');
    host.append(this.referenceLabel);
    this.camera.position.set(17, 22, 25);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.target.copy(this.center);
    this.controls.enableDamping = false;
    this.controls.minDistance = 8;
    this.controls.maxDistance = 55;
    this.controls.maxPolarAngle = Math.PI * 0.43;
    this.controls.minPolarAngle = 0.08;
    this.controls.enablePan = true;
    this.controls.update();
    this.controls.addEventListener('change', () => {
      this.setHovered(null);
      this.invalidate();
    });
    this.renderer.domElement.addEventListener('pointerdown', this.down);
    this.renderer.domElement.addEventListener('pointermove', this.move);
    this.renderer.domElement.addEventListener('pointerleave', this.leave);
    this.renderer.domElement.addEventListener('pointercancel', this.cancel);
    this.renderer.domElement.addEventListener('pointerup', this.up);
    this.renderer.domElement.addEventListener('keydown', this.key);
    this.resize = new ResizeObserver(() => {
      const { width, height } = host.getBoundingClientRect();
      if (!width || !height) return;
      this.renderer.setSize(width, height);
      this.camera.aspect = width / height;
      this.updateViewport();
      this.controls.enableZoom = true;
      this.renderer.domElement.style.touchAction = 'none';
      this.reset();
    });
    this.resize.observe(host);
  }
  private updateViewport() {
    const width = this.host.clientWidth,
      height = this.host.clientHeight;
    if (this.boardInteraction && width <= 760 && height > 400)
      this.camera.setViewOffset(width, height, 0, height * 0.3, width, height);
    else this.camera.clearViewOffset();
    this.camera.updateProjectionMatrix();
    this.invalidate();
  }
  private down = (e: PointerEvent) => {
    this.clearCoordinate();
    this.gesture.down(e.pointerId, e.clientX, e.clientY);
  };
  private up = (e: PointerEvent) => {
    const clicked = this.gesture.up(e.pointerId, e.clientX, e.clientY);
    const hit = this.hitTarget(e.clientX, e.clientY);
    const position = hit?.position ?? null;
    if (clicked && e.button === 0) {
      if (position && this.canPick(position)) this.pick(position, hit?.pick);
      else if (!position) this.dismiss();
    }
    this.setHovered(e.pointerType === 'touch' ? null : position);
  };
  private move = (e: PointerEvent) => {
    this.gesture.move(e.clientX, e.clientY);
    this.setHovered(
      this.gesture.dragging || e.pointerType === 'touch' ? null : this.hitTile(e.clientX, e.clientY),
      true,
    );
  };
  private leave = (e: PointerEvent) => {
    if (e.relatedTarget instanceof Node && this.labelHost.contains(e.relatedTarget)) return;
    this.setHovered(null);
  };
  private cancel = (e: PointerEvent) => {
    this.gesture.cancel(e.pointerId);
    this.setHovered(null);
  };
  private canPick(position: string) {
    return !this.pending && (this.selectable === null || this.selectable.has(position));
  }
  private hitTile(x: number, y: number): string | null {
    return this.hitTarget(x, y)?.position ?? null;
  }
  private hitTarget(x: number, y: number): { position: string; pick: MapPick } | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    this.raycaster.setFromCamera(
      new THREE.Vector2(((x - rect.left) / rect.width) * 2 - 1, (-(y - rect.top) / rect.height) * 2 + 1),
      this.camera,
    );
    const hit = this.raycaster.intersectObjects([...this.tiles.values(), ...this.pieces], true)[0];
    let object: THREE.Object3D | null = hit?.object ?? null;
    while (object) {
      if (typeof object.userData.position === 'string')
        return {
          position: object.userData.position,
          pick: {
            kind: object.userData.kind ?? 'tile',
            player: object.userData.player,
            unit: object.userData.unit,
          },
        };
      object = object.parent;
    }
    return null;
  }
  private setHovered(position: string | null, audible = false) {
    const changed = this.hovered !== position;
    if (audible && changed && position && this.canPick(position) && !this.gesture.dragging) this.hover();
    this.hovered = position;
    this.seaOverlay.setFocus(
      this.seaGuide
        ? this.seaRouteStart
        : this.seaPreviewAllowed && this.seaOverlay.hasWater(position)
          ? position
          : null,
      this.seaGuide,
    );
    this.renderer.domElement.style.cursor = this.gesture.dragging
      ? 'grabbing'
      : this.pending
        ? 'wait'
        : position
          ? this.canPick(position)
            ? 'pointer'
            : 'not-allowed'
          : 'grab';
    this.hoverRing.visible = !!position && this.canPick(position) && !this.gesture.dragging;
    if (position) {
      const [x, z] = positionXY(position);
      this.hoverRing.position.set(x, 0.36, z);
    }
    for (const label of this.labelPositions)
      label.node.classList.toggle('hovered', label.position === position);
    if (changed) this.invalidate();
  }
  private key = (e: KeyboardEvent) => {
    if (e.key === 'Home') this.reset();
    else if (e.key === 'Escape') this.clearCoordinate();
    else if (e.key === '+' || e.key === '=') this.zoom(0.85);
    else if (e.key === '-') this.zoom(1.15);
    else if (e.key.startsWith('Arrow')) {
      e.preventDefault();
      const offset = this.camera.position.clone().sub(this.controls.target);
      offset.applyAxisAngle(
        new THREE.Vector3(0, 1, 0),
        e.key === 'ArrowLeft' ? 0.12 : e.key === 'ArrowRight' ? -0.12 : 0,
      );
      this.camera.position.copy(this.controls.target).add(offset);
      this.controls.update();
      this.invalidate();
    }
  };
  private invalidate() {
    if (this.disposed) return;
    this.dirty = true;
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      if (!this.dirty) return;
      this.dirty = false;
      this.renderer.render(this.scene, this.camera);
      const w = this.host.clientWidth,
        h = this.host.clientHeight;
      for (const label of this.labelPositions) {
        const v = label.at.clone().project(this.camera);
        const cityOffset = this.topDown && label.kind === 'city';
        label.node.style.transform = `translate(-50%, -50%) translate(${((v.x + 1) * w) / 2 + (cityOffset ? 11 : 0)}px,${((-v.y + 1) * h) / 2 + (label.offsetY ?? 0) - (cityOffset ? 12 : 0)}px)`;
        label.node.style.display = v.z > 1 || v.z < -1 ? 'none' : '';
      }
      if (this.referenceRing.visible) {
        const v = this.referenceRing.position.clone().project(this.camera);
        this.referenceLabel.style.transform = `translate(-50%, 8px) translate(${((v.x + 1) * w) / 2}px,${((-v.y + 1) * h) / 2}px)`;
        this.referenceLabel.hidden = v.z > 1;
      }
    });
  }
  highlightCoordinate(position: string | null) {
    const target = position ?? this.pinnedReference;
    const visible = !!target && this.tiles.has(target);
    this.referenceRing.visible = visible;
    this.referenceLabel.hidden = !visible;
    if (visible) {
      const [x, z] = positionXY(target!);
      this.referenceRing.position.set(x, 0.42, z);
      this.referenceLabel.textContent = target;
      this.referenceLabel.setAttribute('aria-label', `Map location ${target}`);
    }
    this.invalidate();
  }
  locateCoordinate(position: string) {
    if (!this.tiles.has(position)) return;
    const [x, z] = positionXY(position);
    const target = new THREE.Vector3(x, 0, z);
    this.camera.position.add(target.clone().sub(this.controls.target));
    this.controls.target.copy(target);
    this.controls.update();
    this.pinnedReference = position;
    this.highlightCoordinate(position);
  }
  clearCoordinate() {
    this.pinnedReference = null;
    this.highlightCoordinate(null);
  }
  zoom(factor: number) {
    const direction = this.camera.position.clone().sub(this.controls.target);
    direction.multiplyScalar(factor).clampLength(8, 55);
    this.camera.position.copy(this.controls.target).add(direction);
    this.controls.update();
    this.invalidate();
  }
  reset() {
    this.clearCoordinate();
    this.renderer.domElement.setAttribute(
      'aria-label',
      `${this.topDown ? 'Overhead' : '3D'} civilization map. Select a city or terrain tile. Drag to ${this.topDown ? 'pan' : 'orbit'}; scroll to zoom.`,
    );
    this.labelHost.classList.toggle('top-down', this.topDown);
    this.controls.enableRotate = !this.topDown;
    this.controls.minPolarAngle = this.topDown ? 0 : 0.08;
    this.controls.maxPolarAngle = this.topDown ? 0 : Math.PI * 0.43;
    this.controls.touches.ONE = this.topDown ? THREE.TOUCH.PAN : THREE.TOUCH.ROTATE;
    this.controls.mouseButtons.LEFT = this.topDown ? THREE.MOUSE.PAN : THREE.MOUSE.ROTATE;
    this.controls.target.copy(this.center);
    this.camera.position
      .copy(this.center)
      .add(
        new THREE.Vector3(
          this.topDown ? 0 : 12.5,
          this.topDown ? 27 : 22,
          this.topDown ? 0.01 : 17,
        ).multiplyScalar(
          Math.max(1, 0.92 / this.camera.aspect) *
            (this.camera.aspect > 1.4 ? 0.85 : 1) *
            (this.seaGuide ? 1.15 : 1),
        ),
      );
    this.controls.update();
    this.invalidate();
  }
  private building(color: string) {
    const group = new THREE.Group();
    const stone = this.material('#ead7ad'),
      roof = this.material('#a86042');
    const base = this.mesh(new THREE.BoxGeometry(0.83, 0.12, 0.66), stone);
    base.position.y = 0.06;
    group.add(base);
    const hall = this.mesh(new THREE.BoxGeometry(0.54, 0.3, 0.4), stone);
    hall.position.set(0, 0.26, 0);
    group.add(hall);
    const ridge = this.mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.66, 3), roof);
    ridge.rotation.z = Math.PI / 2;
    ridge.rotation.x = Math.PI / 2;
    ridge.position.y = 0.51;
    group.add(ridge);
    for (const x of [-0.25, -0.08, 0.09, 0.26]) {
      const column = this.mesh(new THREE.CylinderGeometry(0.033, 0.04, 0.32, 7), stone);
      column.position.set(x, 0.28, 0.29);
      group.add(column);
    }
    for (let i = 0; i < 3; i++) {
      const step = this.mesh(new THREE.BoxGeometry(0.68 + i * 0.08, 0.04, 0.16), stone);
      step.position.set(0, 0.12 - i * 0.025, 0.37 + i * 0.06);
      group.add(step);
    }
    const pole = this.mesh(new THREE.CylinderGeometry(0.015, 0.02, 0.85, 6), this.material('#624d30'));
    pole.position.set(0.47, 0.5, -0.2);
    group.add(pole);
    const flag = this.mesh(new THREE.BoxGeometry(0.28, 0.18, 0.018), this.material(color));
    flag.position.set(0.6, 0.84, -0.2);
    group.add(flag);
    return group;
  }
  private tree(x: number, z: number, scale: number, seed: number) {
    const group = new THREE.Group();
    const trunk = this.mesh(new THREE.CylinderGeometry(0.035, 0.055, 0.4, 5), this.material('#6b6042'));
    trunk.position.y = 0.2;
    group.add(trunk);
    const green = this.material(['#355849', '#456d4f', '#637950'][seed % 3]);
    for (let k = 0; k < 3; k++) {
      const cone = this.mesh(new THREE.ConeGeometry(0.22 - k * 0.035, 0.45, 6), green);
      cone.position.y = 0.39 + k * 0.18;
      group.add(cone);
    }
    group.position.set(x, 0.12, z);
    group.scale.setScalar(scale);
    return group;
  }
  private addTerrain(group: THREE.Group, kind: string, seed: number) {
    if (kind === 'Forest') {
      for (let i = 0; i < 9; i++) {
        const angle = i * 2.4;
        const radius = 0.18 + 0.1 * (i % 5);
        group.add(
          this.tree(Math.cos(angle) * radius, Math.sin(angle) * radius, 0.75 + (i % 3) * 0.17, i + seed),
        );
      }
    }
    if (kind === 'Mountain') {
      for (let i = 0; i < 3; i++) {
        const height = [1.1, 0.8, 0.6][i];
        const rock = this.mesh(
          new THREE.ConeGeometry(0.45, height, 5),
          this.material(['#82867a', '#999c8c', '#767d77'][i]),
        );
        rock.position.set((i - 1) * 0.32, height / 2 + 0.08, (i % 2) * 0.24 - 0.1);
        rock.rotation.y = i;
        group.add(rock);
        const cap = this.mesh(new THREE.ConeGeometry(0.14, height * 0.28, 5), this.material('#eee9d7'));
        cap.position.copy(rock.position);
        cap.position.y = height * 0.87 + 0.08;
        cap.rotation.y = i;
        group.add(cap);
      }
    }
    if (kind === 'Fertile') {
      const fieldMat = this.material('#c6b968');
      for (let i = 0; i < 5; i++) {
        const furrow = this.mesh(new THREE.BoxGeometry(0.06, 0.035, 0.7), fieldMat);
        furrow.position.set(-0.35 + i * 0.14, 0.12, 0.03);
        furrow.rotation.y = 0.25;
        group.add(furrow);
      }
      group.add(this.tree(0.55, -0.25, 0.5, seed));
    }
    if (kind === 'Barren') {
      for (let i = 0; i < 7; i++) {
        const rock = this.mesh(
          new THREE.DodecahedronGeometry(0.09 + (i % 3) * 0.04, 0),
          this.material('#c8b18b'),
        );
        rock.position.set(Math.sin(i * 8) * 0.65, 0.13, Math.cos(i * 5) * 0.6);
        rock.scale.y = 0.5;
        group.add(rock);
      }
    }
    if (kind === 'Unexplored') {
      const mat = this.material('#90aba5');
      const circle = this.mesh(new THREE.TorusGeometry(0.22, 0.009, 4, 24), mat);
      circle.rotation.x = -Math.PI / 2;
      circle.position.y = 0.09;
      group.add(circle);
      for (let i = 0; i < 4; i++) {
        const mark = this.mesh(new THREE.BoxGeometry(0.014, 0.012, 0.14), mat);
        mark.position.set(Math.sin((i * Math.PI) / 2) * 0.26, 0.093, Math.cos((i * Math.PI) / 2) * 0.26);
        mark.rotation.y = (i * Math.PI) / 2;
        group.add(mark);
      }
    }
    if (kind === 'Water') {
      for (let i = 0; i < 4; i++) {
        const line = this.mesh(new THREE.BoxGeometry(0.5 - i * 0.08, 0.018, 0.025), this.material('#96c9c1'));
        line.position.set((i % 2) * 0.16, 0.07, (i - 2) * 0.19);
        group.add(line);
      }
    }
  }
  private ownershipBadge(index: number) {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 64;
    const context = canvas.getContext('2d')!;
    context.fillStyle = '#fff7e6';
    context.strokeStyle = '#2d392f';
    context.lineWidth = 3;
    context.beginPath();
    context.arc(32, 32, 29, 0, Math.PI * 2);
    context.fill();
    context.stroke();
    context.fillStyle = '#25372e';
    context.font = 'bold 42px system-ui';
    context.textAlign = 'center';
    context.textBaseline = 'middle';
    context.fillText(playerSymbol(index), 32, 33);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    this.textures.add(texture);
    const material = new THREE.SpriteMaterial({ map: texture, depthTest: false, toneMapped: false });
    this.materials.add(material);
    const badge = new THREE.Sprite(material);
    badge.scale.set(0.42, 0.42, 1);
    badge.renderOrder = 2;
    return badge;
  }
  private clearCollectionBadges() {
    for (const badge of this.collectionBadges.values()) {
      void unmount(badge.component);
      badge.node.remove();
    }
    this.collectionBadges.clear();
    this.collectionSignature = '';
    this.labelPositions = this.labelPositions.filter((l) => l.kind !== 'collection');
  }
  private clearBoard() {
    this.clearCollectionBadges();
    for (const badge of this.unitBadges) void unmount(badge);
    this.unitBadges = [];
    this.board.traverse((o) => {
      if (o instanceof THREE.Mesh || o instanceof THREE.Sprite) {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          this.geometries.delete(o.geometry);
        }
        const mats = Array.isArray(o.material) ? o.material : [o.material];
        mats.forEach((m) => {
          if (m instanceof THREE.SpriteMaterial && m.map) {
            m.map.dispose();
            this.textures.delete(m.map);
          }
          m.dispose();
          this.materials.delete(m);
        });
      }
    });
    this.board.clear();
    this.tiles.clear();
    this.pieces = [];
    this.labelHost.replaceChildren();
    this.labelPositions = [];
    this.moveMarkerSignature = '';
  }
  update(s: Session) {
    if (!s.game) return;
    const interacting = s.mode === 'collect' || s.mode === 'settlers' || s.tilePanel;
    if (interacting !== this.boardInteraction) {
      this.boardInteraction = interacting;
      this.updateViewport();
    }
    const seaGuide = s.seaRoutes && s.mode === 'overview';
    const guideChanged = seaGuide !== this.seaGuide;
    this.seaGuide = seaGuide;
    this.seaPreviewAllowed =
      s.mode === 'overview' &&
      !s.pending &&
      !s.view?.decision &&
      !s.view?.explorationDecision &&
      !s.view?.choiceDecision &&
      !s.view?.objectiveDecision;
    this.seaRouteStart = s.seaRouteStart;
    this.seaOverlay.update(s.game.map.tiles, s.view?.seaRoutes ?? []);
    this.pending = s.pending;
    const exploration = s.view?.explorationDecision;
    const placement =
      exploration?.choices.find(
        (choice) => choice.rotation === (s.explorationPreview ?? s.explorationRotation),
      ) ?? exploration?.choices[0];
    const overlay = new Map(placement?.tiles ?? []);
    const mapTiles: typeof s.game.map.tiles = s.game.map.tiles.map(([position, terrain]) => [
      position,
      overlay.get(position) ?? terrain,
    ]);
    this.selectable = exploration
      ? new Set()
      : s.mode === 'collect'
        ? new Set(s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ?? [])
        : s.mode === 'settlers'
          ? new Set([
              ...(s.view?.units?.map((u) => u.position) ?? []),
              ...s.moveDestinations.map((d) => d.position),
            ])
          : null;
    const signature = JSON.stringify([
      mapTiles,
      s.game.players.map((p) => [p.cities, p.units]),
      s.view?.players,
      s.seat,
      s.colorBlind,
    ]);
    if (signature !== this.lastSignature) {
      this.lastSignature = signature;
      this.clearBoard();
      const coords = s.game.map.tiles.map(([p]) => positionXY(p));
      const minX = Math.min(...coords.map((c) => c[0])),
        maxX = Math.max(...coords.map((c) => c[0])),
        minZ = Math.min(...coords.map((c) => c[1])),
        maxZ = Math.max(...coords.map((c) => c[1]));
      this.center.set((minX + maxX) / 2, 0, (minZ + maxZ) / 2);
      for (const [position, terrain] of mapTiles) {
        const kind = typeof terrain === 'string' ? terrain : 'Barren';
        const [x, z] = positionXY(position);
        const height = kind === 'Water' ? 0.13 : kind === 'Unexplored' ? 0.27 : 0.43;
        const group = new THREE.Group();
        group.position.set(x, height / 2 - 0.12, z);
        const top = this.material(terrainColor[kind]);
        const side = this.material(
          kind === 'Unexplored' ? '#4c7378' : kind === 'Water' ? '#4c8a91' : '#837655',
        );
        const hex = this.mesh(new THREE.CylinderGeometry(0.976, 0.98, height, 6), [side, top, side]);
        hex.rotation.y = Math.PI / 6;
        hex.userData.position = position;
        group.add(hex);
        this.tiles.set(position, hex);
        const terrainGroup = new THREE.Group();
        terrainGroup.position.y = height / 2;
        this.addTerrain(terrainGroup, kind, position.charCodeAt(0));
        group.add(terrainGroup);
        this.board.add(group);
      }
      for (const player of s.game.players) {
        for (const city of player.cities ?? []) {
          const [x, z] = positionXY(city.position);
          const cityModel = this.building(playerColor(player.id, s.colorBlind));
          cityModel.position.set(x, 0.38, z);
          cityModel.rotation.y = 0.25;
          const capital = s.view?.players.find((p) => p.index === player.id)?.capital === city.position;
          if (capital) {
            const gold = this.material('#d5af55');
            const band = this.mesh(new THREE.CylinderGeometry(0.16, 0.16, 0.07, 12), gold);
            band.position.set(0, 0.91, 0);
            cityModel.add(band);
            for (let i = 0; i < 3; i++) {
              const point = this.mesh(new THREE.ConeGeometry(0.055, 0.15, 4), gold);
              point.position.set((i - 1) * 0.11, 1.01, 0);
              cityModel.add(point);
            }
          }
          cityModel.userData = { position: city.position, kind: 'city', player: player.id };
          this.pieces.push(cityModel);
          if (s.colorBlind) {
            const badge = this.ownershipBadge(player.id);
            badge.position.set(0.6, 1.05, -0.2);
            cityModel.add(badge);
          }
          const additions = Object.keys(city.city_pieces ?? {}).filter((k) => k !== 'wonders');
          for (const [j, name] of additions.entries()) {
            const annex = new THREE.Group();
            const stone = this.material('#d5c6a0');
            const angle = j * 2.4;
            const tower = this.mesh(
              name === 'obelisk'
                ? new THREE.ConeGeometry(0.1, 0.85, 4)
                : new THREE.BoxGeometry(0.22, name === 'fortress' ? 0.55 : 0.27, 0.26),
              stone,
            );
            tower.position.y = name === 'obelisk' ? 0.42 : name === 'fortress' ? 0.27 : 0.14;
            annex.add(tower);
            if (name !== 'obelisk' && name !== 'fortress') {
              const roof = this.mesh(
                new THREE.ConeGeometry(0.23, 0.18, name === 'observatory' ? 12 : 4),
                this.material(name === 'observatory' ? '#62887b' : '#a86042'),
              );
              roof.position.y = 0.35;
              annex.add(roof);
            }
            annex.position.set(Math.cos(angle) * 0.66, 0, Math.sin(angle) * 0.64);
            cityModel.add(annex);
          }
          this.board.add(cityModel);
          const label = document.createElement('button');
          label.className = 'city-map-label';
          label.style.setProperty('--player-color', playerColor(player.id, s.colorBlind));
          const mood = city.mood_state;
          label.dataset.mood = mood.toLowerCase();
          const face = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
          face.setAttribute('viewBox', '0 0 24 24');
          face.setAttribute('aria-hidden', 'true');
          face.setAttribute('fill', 'none');
          face.setAttribute('stroke', 'currentColor');
          face.setAttribute('stroke-width', '1.8');
          face.setAttribute('stroke-linecap', 'round');
          face.setAttribute('stroke-linejoin', 'round');
          const outline = document.createElementNS(face.namespaceURI, 'circle');
          outline.setAttribute('cx', '12');
          outline.setAttribute('cy', '12');
          outline.setAttribute('r', '9');
          const features = document.createElementNS(face.namespaceURI, 'path');
          features.setAttribute(
            'd',
            `M8 9h.01M16 9h.01 ${mood === 'Happy' ? 'M8 14q4 4 8 0' : mood === 'Angry' ? 'M8 17q4-4 8 0' : 'M8 15h8'}`,
          );
          face.append(outline, features);
          const name = document.createElement('span');
          name.className = 'city-map-name';
          name.textContent = `${s.colorBlind ? playerSymbol(player.id) + ' ' : ''}${player.civilization}${capital ? ' ♛' : ''} · ${city.position} · ${mood}`;
          label.append(face, name);
          label.setAttribute('aria-label', `Select ${player.civilization} city ${city.position} · ${mood}`);
          label.onclick = () => {
            if (this.canPick(city.position)) this.pick(city.position, { kind: 'city', player: player.id });
          };
          label.onpointerenter = () => this.setHovered(city.position, true);
          label.onpointerleave = () => this.setHovered(null);
          label.onfocus = () => this.setHovered(city.position);
          label.onblur = () => this.setHovered(null);
          this.labelHost.append(label);
          this.labelPositions.push({
            position: city.position,
            at: new THREE.Vector3(x, 1.6, z),
            node: label,
            kind: 'city',
          });
        }
        for (const [unitIndex, unit] of (player.units ?? []).entries()) {
          const [x, z] = positionXY(unit.position);
          const pawn = new THREE.Group();
          pawn.userData = { position: unit.position, kind: 'unit', unit: unit.id, player: player.id };
          this.pieces.push(pawn);
          const color = this.material(playerColor(player.id, s.colorBlind));
          const body = this.mesh(new THREE.ConeGeometry(0.085, 0.27, 7), color);
          body.position.y = 0.19;
          pawn.add(body);
          const head = this.mesh(new THREE.SphereGeometry(0.062, 8, 6), this.material('#e4c7a1'));
          head.position.y = 0.39;
          pawn.add(head);
          const base = this.mesh(new THREE.CylinderGeometry(0.14, 0.14, 0.04, 14), this.material('#e5d19e'));
          pawn.add(base);
          if (unit.unit_type === 'Infantry') {
            const spear = this.mesh(
              new THREE.CylinderGeometry(0.013, 0.013, 0.56, 5),
              this.material('#6b6042'),
            );
            spear.position.set(0.13, 0.28, 0);
            pawn.add(spear);
          } else if (unit.unit_type === 'Ship') {
            const hull = this.mesh(new THREE.BoxGeometry(0.23, 0.13, 0.5), color);
            hull.position.y = 0.07;
            pawn.add(hull);
            const sail = this.mesh(new THREE.BoxGeometry(0.25, 0.27, 0.018), this.material('#f4e8cc'));
            sail.position.y = 0.4;
            pawn.add(sail);
          } else if (unit.unit_type === 'Cavalry' || unit.unit_type === 'Elephant') {
            const mount = this.mesh(
              new THREE.BoxGeometry(0.19, 0.2, 0.35),
              this.material(unit.unit_type === 'Elephant' ? '#859187' : '#887258'),
            );
            mount.position.y = 0.13;
            pawn.add(mount);
          } else if (typeof unit.unit_type === 'object') {
            const crown = this.mesh(
              new THREE.CylinderGeometry(0.09, 0.075, 0.09, 6),
              this.material('#d5af55'),
            );
            crown.position.y = 0.46;
            pawn.add(crown);
          }
          const stackIndex = (player.units ?? [])
            .slice(0, unitIndex)
            .filter((u) => u.position === unit.position).length;
          pawn.position.set(
            x - 0.58 + (stackIndex % 3) * 0.23,
            0.4,
            z + 0.25 + Math.floor(stackIndex / 3) * 0.23,
          );
          this.board.add(pawn);
        }
      }
      const stackRows = new Map<string, number>();
      for (const player of s.game.players) {
        for (const position of new Set(player.units?.map((unit) => unit.position))) {
          if (!this.tiles.has(position)) continue;
          const surfaceUnits = player.units!.filter((unit) => unit.position === position);
          const units = surfaceUnits.flatMap((unit) => [
            unit,
            ...(unit.carried_units ?? []).map((carried) => ({ ...carried, position })),
          ]);
          const counts = new Map<string, { type: (typeof units)[number]['unit_type']; count: number }>();
          for (const unit of units) {
            const name = typeof unit.unit_type === 'string' ? unit.unit_type : unit.unit_type.Leader;
            const group = counts.get(name) ?? { type: unit.unit_type, count: 0 };
            group.count++;
            counts.set(name, group);
          }
          const carried = surfaceUnits.reduce((sum, unit) => sum + (unit.carried_units?.length ?? 0), 0);
          const description = `${player.civilization} · ${position}: ${[...counts].map(([name, group]) => `${group.count} ${name}`).join(', ')}${carried ? ` · ${carried} aboard ships` : ''}`;
          const label = document.createElement('button');
          label.className = 'unit-map-label';
          label.style.setProperty('--player-color', playerColor(player.id, s.colorBlind));
          label.setAttribute('aria-label', `Inspect ${description}`);
          label.title = description;
          this.unitBadges.push(
            mount(UnitMapBadge, {
              target: label,
              props: { groups: [...counts.values()], symbol: s.colorBlind ? playerSymbol(player.id) : '' },
            }),
          );
          label.onclick = () => {
            if (this.canPick(position)) this.pick(position, { kind: 'units', player: player.id });
          };
          label.onpointerenter = () => this.setHovered(position, true);
          label.onpointerleave = () => this.setHovered(null);
          label.onfocus = () => this.setHovered(position);
          label.onblur = () => this.setHovered(null);
          const [x, z] = positionXY(position);
          const row = stackRows.get(position) ?? 0;
          stackRows.set(position, row + 1);
          this.labelHost.append(label);
          this.labelPositions.push({
            position,
            at: new THREE.Vector3(x, 0.5, z),
            node: label,
            kind: 'units',
            offsetY: 15 + row * 29,
          });
        }
      }
      if (this.selectionSignature === '') this.reset();
    }
    const settler = s.view?.units?.find((u) => s.selectedUnits.includes(u.id));
    const focusedPosition =
      s.mode === 'settlers' ? settler?.position : s.mode === 'overview' ? (s.focus ?? s.city) : s.city;
    const selected = exploration
      ? exploration.destination
        ? [exploration.destination]
        : []
      : s.mode === 'settlers'
        ? [settler?.position, s.moveTarget].filter((p): p is string => !!p)
        : s.mode === 'collect'
          ? s.selection.map((c) => c.position)
          : s.tilePanel && focusedPosition
            ? [focusedPosition]
            : [];
    const available = placement
      ? placement.tiles.map(([position]) => position)
      : s.mode === 'settlers'
        ? s.moveDestinations.map((d) => d.position)
        : s.mode === 'collect'
          ? (s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ?? [])
          : [];
    const selectionSig = JSON.stringify([selected, available]);
    if (selectionSig !== this.selectionSignature) {
      this.selectionSignature = selectionSig;
      this.rings.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          this.geometries.delete(o.geometry);
          (o.material as THREE.Material).dispose();
          this.materials.delete(o.material as THREE.Material);
        }
      });
      this.rings.clear();
      for (const pos of new Set([...selected, ...available])) {
        const [x, z] = positionXY(pos);
        const ring = this.mesh(
          new THREE.TorusGeometry(0.99, selected.includes(pos) ? 0.06 : 0.018, 6, 6),
          new THREE.MeshBasicMaterial({ color: selected.includes(pos) ? '#ffd16b' : '#ebdab3' }),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.position.set(x, selected.includes(pos) ? 0.48 : 0.35, z);
        this.materials.add(ring.material as THREE.Material);
        this.rings.add(ring);
        if (selected.includes(pos)) {
          const material = new THREE.MeshBasicMaterial({
            color: '#ffc450',
            transparent: true,
            opacity: 0.27,
            depthWrite: false,
            side: THREE.DoubleSide,
          });
          this.materials.add(material);
          const fill = this.mesh(new THREE.CircleGeometry(0.92, 6), material);
          fill.rotation.x = -Math.PI / 2;
          fill.position.set(x, 0.47, z);
          fill.castShadow = false;
          this.rings.add(fill);
        }
      }
    }
    const collectionChoices =
      s.mode === 'collect' ? (s.view?.cities.find((c) => c.position === s.city)?.choices ?? []) : [];
    const collectionSignature = JSON.stringify([collectionChoices, s.selection]);
    if (collectionSignature !== this.collectionSignature) {
      const positions = new Set(collectionChoices.map((c) => c.position));
      for (const [position, badge] of this.collectionBadges) {
        if (!positions.has(position)) {
          void unmount(badge.component);
          badge.node.remove();
          this.collectionBadges.delete(position);
        }
      }
      this.labelPositions = this.labelPositions.filter(
        (l) => l.kind !== 'collection' || positions.has(l.position),
      );
      for (const position of positions) {
        const choices = collectionChoices.filter((c) => c.position === position);
        const selectedChoices = choices.filter((c) =>
          s.selection.some((selected) => sameCollection(c, selected)),
        );
        const displayed = selectedChoices.length ? selectedChoices : choices;
        const piles = displayed.map((c) => collectionYield(c, s.selection));
        const previous = this.collectionBadges.get(position);
        const label = previous?.node ?? document.createElement('button');
        if (previous) void unmount(previous.component);
        else {
          label.className = 'collect-map-label';
          label.onclick = () => {
            if (this.canPick(position)) this.pick(position);
          };
          label.onpointerenter = () => this.setHovered(position, true);
          label.onpointerleave = () => this.setHovered(null);
          label.onfocus = () => this.setHovered(position);
          label.onblur = () => this.setHovered(null);
          this.labelHost.append(label);
          const [x, z] = positionXY(position);
          this.labelPositions.push({
            position,
            at: new THREE.Vector3(x, 0.7, z),
            node: label,
            kind: 'collection',
          });
        }
        const amounts = piles
          .map((pile) =>
            Object.entries(pile)
              .map(([r, n]) => `${n} ${r.replace('_tokens', ' tokens')}`)
              .join(' + '),
          )
          .join(' or ');
        const bonuses = [...new Set(choices.map(collectionBonusLabel).filter(Boolean))].join(', ');
        label.title = `${position}: ${amounts}${bonuses ? ` · ${bonuses}` : ''}`;
        label.setAttribute('aria-label', `Collect at ${label.title}`);
        label.setAttribute('aria-pressed', String(selectedChoices.length > 0));
        label.classList.toggle('selected', selectedChoices.length > 0);
        this.collectionBadges.set(position, {
          node: label,
          component: mount(CollectionMapBadge, {
            target: label,
            props: { piles, selected: selectedChoices.length > 0 },
          }),
        });
      }
      this.collectionSignature = collectionSignature;
    }
    const moveMarkers = s.mode === 'settlers' ? [...new Set(s.moveDestinations.map((d) => d.position))] : [];
    const markerSignature = JSON.stringify(moveMarkers);
    if (markerSignature !== this.moveMarkerSignature) {
      this.moveMarkerSignature = markerSignature;
      for (const label of this.labelPositions.filter((l) => l.kind === 'destination')) label.node.remove();
      this.labelPositions = this.labelPositions.filter((l) => l.kind !== 'destination');
      for (const position of moveMarkers) {
        const label = document.createElement('button');
        label.className = 'move-map-label';
        label.textContent = position;
        label.setAttribute('aria-label', `Move destination ${position}`);
        label.title = `Select destination ${position}`;
        label.onclick = () => {
          if (this.canPick(position)) this.pick(position);
        };
        label.onpointerenter = () => this.setHovered(position, true);
        label.onpointerleave = () => this.setHovered(null);
        label.onfocus = () => this.setHovered(position);
        label.onblur = () => this.setHovered(null);
        this.labelHost.append(label);
        const [x, z] = positionXY(position);
        this.labelPositions.push({
          position,
          at: new THREE.Vector3(x, 0.7, z),
          node: label,
          kind: 'destination',
        });
      }
    }
    if (seaGuide) {
      this.selectable = new Set(
        s.game.map.tiles.filter(([, terrain]) => terrain === 'Water').map(([p]) => p),
      );
      this.rings.visible = false;
    } else this.rings.visible = true;
    this.labelHost.classList.toggle('hide-unit-badges', !s.unitBadges);
    for (const label of this.labelPositions) {
      if (label.kind !== 'collection')
        label.node.classList.toggle(
          'selected',
          label.position === (label.kind === 'destination' ? s.moveTarget : focusedPosition),
        );
      label.node.disabled = !this.canPick(label.position);
    }
    if (s.topDown !== this.topDown || guideChanged) {
      this.topDown = s.topDown;
      this.reset();
    }
    this.setHovered(this.hovered);
    this.invalidate();
  }
  destroy() {
    this.disposed = true;
    this.clearCollectionBadges();
    for (const badge of this.unitBadges) void unmount(badge);
    this.unitBadges = [];
    this.seaOverlay.dispose();
    cancelAnimationFrame(this.frame);
    this.resize.disconnect();
    this.controls.dispose();
    this.renderer.domElement.removeEventListener('pointerdown', this.down);
    this.renderer.domElement.removeEventListener('pointermove', this.move);
    this.renderer.domElement.removeEventListener('pointerleave', this.leave);
    this.renderer.domElement.removeEventListener('pointercancel', this.cancel);
    this.renderer.domElement.removeEventListener('pointerup', this.up);
    this.renderer.domElement.removeEventListener('keydown', this.key);
    for (const geometry of this.geometries) geometry.dispose();
    for (const material of this.materials) material.dispose();
    for (const texture of this.textures) texture.dispose();
    this.renderer.dispose();
    this.renderer.domElement.remove();
    this.labelHost.remove();
    this.referenceLabel.remove();
  }
}
