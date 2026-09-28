import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Session, Terrain } from './types';
import { playerColor, playerSymbol } from './types';
import { positionXY } from './model';
import { MapGesture } from './map-gesture';

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
  private tiles = new Map<string, THREE.Mesh>();
  private pieces: THREE.Group[] = [];
  private hoverRing: THREE.Mesh;
  private hovered: string | null = null;
  private selectable: Set<string> | null = null;
  private pending = false;
  private gesture = new MapGesture();
  private raycaster = new THREE.Raycaster();
  private resize: ResizeObserver;
  private frame = 0;
  private dirty = true;
  private lastSignature = '';
  private selectionSignature = '';
  private disposed = false;
  private topDown = false;
  private materials = new Set<THREE.Material>();
  private geometries = new Set<THREE.BufferGeometry>();
  private textures = new Set<THREE.Texture>();
  private labelPositions: { position: string; at: THREE.Vector3; node: HTMLButtonElement }[] = [];
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
    private pick: (position: string) => void,
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
      this.camera.updateProjectionMatrix();
      this.controls.enableZoom = true;
      this.renderer.domElement.style.touchAction = 'none';
      this.reset();
    });
    this.resize.observe(host);
  }
  private down = (e: PointerEvent) => {
    this.gesture.down(e.pointerId, e.clientX, e.clientY);
  };
  private up = (e: PointerEvent) => {
    const clicked = this.gesture.up(e.pointerId, e.clientX, e.clientY);
    const position = this.hitTile(e.clientX, e.clientY);
    if (clicked && e.button === 0 && position && this.canPick(position)) this.pick(position);
    this.setHovered(e.pointerType === 'touch' ? null : position);
  };
  private move = (e: PointerEvent) => {
    this.gesture.move(e.clientX, e.clientY);
    this.setHovered(
      this.gesture.dragging || e.pointerType === 'touch' ? null : this.hitTile(e.clientX, e.clientY),
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
    const rect = this.renderer.domElement.getBoundingClientRect();
    this.raycaster.setFromCamera(
      new THREE.Vector2(((x - rect.left) / rect.width) * 2 - 1, (-(y - rect.top) / rect.height) * 2 + 1),
      this.camera,
    );
    const hit = this.raycaster.intersectObjects([...this.tiles.values(), ...this.pieces], true)[0];
    let object: THREE.Object3D | null = hit?.object ?? null;
    while (object) {
      if (typeof object.userData.position === 'string') return object.userData.position;
      object = object.parent;
    }
    return null;
  }
  private setHovered(position: string | null) {
    const changed = this.hovered !== position;
    this.hovered = position;
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
        label.node.style.transform = `translate(-50%, -50%) translate(${((v.x + 1) * w) / 2}px,${((-v.y + 1) * h) / 2}px)`;
        label.node.style.display = v.z > 1 ? 'none' : '';
      }
    });
  }
  zoom(factor: number) {
    const direction = this.camera.position.clone().sub(this.controls.target);
    direction.multiplyScalar(factor).clampLength(8, 55);
    this.camera.position.copy(this.controls.target).add(direction);
    this.controls.update();
    this.invalidate();
  }
  reset() {
    this.renderer.domElement.setAttribute(
      'aria-label',
      `${this.topDown ? 'Overhead' : '3D'} civilization map. Select a city or terrain tile. Drag to ${this.topDown ? 'pan' : 'orbit'}; scroll to zoom.`,
    );
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
        ).multiplyScalar(Math.max(1, 0.92 / this.camera.aspect) * (this.camera.aspect > 1.4 ? 0.85 : 1)),
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
  private clearBoard() {
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
  }
  update(s: Session) {
    if (!s.game) return;
    this.pending = s.pending;
    this.selectable =
      s.mode === 'collect'
        ? new Set(s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ?? [])
        : s.mode === 'settlers'
          ? new Set([
              ...(s.view?.settlers.map((u) => u.position) ?? []),
              ...(s.view?.settlers
                .find((u) => u.id === s.selectedSettler)
                ?.destinations.map((d) => d.position) ?? []),
            ])
          : null;
    const signature = JSON.stringify([
      s.game.map.tiles,
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
      for (const [position, terrain] of s.game.map.tiles) {
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
          cityModel.userData.position = city.position;
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
          label.textContent = `${s.colorBlind ? playerSymbol(player.id) + ' ' : ''}${player.civilization} · ${city.position}`;
          label.setAttribute('aria-label', `Select ${player.civilization} city ${city.position}`);
          label.onclick = () => {
            if (this.canPick(city.position)) this.pick(city.position);
          };
          label.onpointerenter = () => this.setHovered(city.position);
          label.onpointerleave = () => this.setHovered(null);
          label.onfocus = () => this.setHovered(city.position);
          label.onblur = () => this.setHovered(null);
          this.labelHost.append(label);
          this.labelPositions.push({
            position: city.position,
            at: new THREE.Vector3(x, 1.85, z),
            node: label,
          });
        }
        for (const [unitIndex, unit] of (player.units ?? []).entries()) {
          const [x, z] = positionXY(unit.position);
          const pawn = new THREE.Group();
          pawn.userData.position = unit.position;
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
          }
          const stackIndex = (player.units ?? [])
            .slice(0, unitIndex)
            .filter((u) => u.position === unit.position).length;
          if (s.colorBlind && stackIndex === 0) {
            const badge = this.ownershipBadge(player.id);
            badge.position.set(-0.08, 0.65, 0);
            pawn.add(badge);
          }
          pawn.position.set(
            x - 0.58 + (stackIndex % 3) * 0.23,
            0.4,
            z + 0.25 + Math.floor(stackIndex / 3) * 0.23,
          );
          this.board.add(pawn);
        }
      }
      if (this.selectionSignature === '') this.reset();
    }
    const settler = s.view?.settlers.find((u) => u.id === s.selectedSettler);
    const focusedPosition = s.mode === 'overview' ? (s.focus ?? s.city) : s.city;
    const selected =
      s.mode === 'settlers'
        ? [settler?.position, s.destination].filter((p): p is string => !!p)
        : s.mode === 'collect'
          ? s.selection.map((c) => c.position)
          : focusedPosition
            ? [focusedPosition]
            : [];
    const available =
      s.mode === 'settlers'
        ? (settler?.destinations.map((d) => d.position) ?? [])
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
          new THREE.TorusGeometry(0.99, selected.includes(pos) ? 0.035 : 0.018, 6, 6),
          this.material(selected.includes(pos) ? '#ffe0a0' : '#e5cc92'),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.position.set(x, 0.35, z);
        this.rings.add(ring);
      }
    }
    for (const label of this.labelPositions) {
      label.node.classList.toggle('selected', label.position === focusedPosition);
      label.node.disabled = !this.canPick(label.position);
    }
    if (s.topDown !== this.topDown) {
      this.topDown = s.topDown;
      this.reset();
    }
    this.setHovered(this.hovered);
    this.invalidate();
  }
  destroy() {
    this.disposed = true;
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
  }
}
