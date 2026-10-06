import { thumbnailCamera } from './thumbnail-camera';
import CollectionMapBadge from './CollectionMapBadge.svelte';
import {
  collectionYield,
  collectionBonusLabel,
  collectionBonusIndicators,
  collectionTriggerLabel,
  collectionStorageWaste,
  sameCollection,
} from './collection-yield';
import type { MapPick } from './types';
import * as THREE from 'three';
import { activeCityAbility } from './abilities';
import { happinessCities } from './happiness';
import { ExplorationOverlay } from './exploration-overlay';
import { explorationPreview } from './exploration-preview';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Session, Terrain } from './types';
import { playerColor, playerSymbol } from './types';
import { positionXY } from './model';
import { terrainInfo } from './terrain';
import { PieceModels, type BuildingKind } from './piece-models';
import { MapGesture } from './map-gesture';
import { SeaOverlay } from './sea-overlay';
import { CombatOverlay } from './combat-overlay';
import { PlacementAnimation } from './placement-animation';
import { activeCombat } from './active-combat';
import { Swords } from 'lucide-svelte';
import { mount, unmount } from 'svelte';
import UnitMapBadge from './UnitMapBadge.svelte';
import { mapDecisionOptions } from './decision-controls';
import { portPlacement } from './port-layout';
import { frameDetails } from './playback';
import { strategyTiles, strategyDescription } from './strategy';
import StrategyMapTile from './StrategyMapTile.svelte';
import { CivilizationFlags } from './civilization-flags';
import { TileTooltip } from './tile-hover';
import { strategyHome, strategyFrame } from './strategy-camera';
import { ResourceOverlay } from './resource-overlay';
import { frameResources } from './resource-playback';
import { publicActions } from './replay-actions';

const terrainColor: Record<string, string> = {
  Forest: '#54755a',
  Fertile: '#9aa36b',
  Mountain: '#858c87',
  Barren: '#b6a27c',
  Water: '#438d95',
  Unexplored: '#526f77',
};
export class World {
  private resourceOverlay: ResourceOverlay;
  private resourceCursor = -1;
  private resourceHistory = '';
  private resourcePlayback = false;
  private explorationOverlay = new ExplorationOverlay();
  private explorationLabel: HTMLDivElement;
  private explorationPositions: string[] = [];
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(36, 1, 0.1, 120);
  private renderer: THREE.WebGLRenderer;
  private controls: OrbitControls;
  private board = new THREE.Group();
  private rings = new THREE.Group();
  private seaOverlay = new SeaOverlay();
  private combatOverlay = new CombatOverlay();
  private combatLabel: HTMLDivElement;
  private combatLabelText: Text;
  private combatIcon: ReturnType<typeof mount>;
  private battleKey = '';
  private seaGuide = false;
  private seaPreviewAllowed = false;
  private seaRouteStart: string | null = null;
  private tiles = new Map<string, THREE.Mesh>();
  private pieces: THREE.Group[] = [];
  private buildings = new Map<string, THREE.Group>();
  private placement = new PlacementAnimation();
  private motionFrame = 0;
  private settleMotion: (() => void) | null = null;
  private lastBoardCursor = '';
  private lastBoardFrame = -1;
  private lastBoardHistory = '';
  private wasPlayback = false;
  private pieceKey(piece: THREE.Group) {
    const d = piece.userData;
    return `${d.kind}:${d.player}:${d.unit ?? d.cityPosition ?? d.position}:${d.building ?? ''}`;
  }
  private animatePieces(previous: Map<string, THREE.Vector3>, appearing: Set<THREE.Group>) {
    const moves = this.pieces
      .filter((piece) => !appearing.has(piece))
      .map((piece) => ({
        piece,
        from: previous.get(this.pieceKey(piece)),
        to: piece.position.clone(),
        scale: piece.scale.clone(),
      }))
      .filter((m) => m.from && m.from.distanceTo(m.to) > 0.02);
    if (!moves.length) return;
    const start = performance.now();
    this.settleMotion = () => {
      cancelAnimationFrame(this.motionFrame);
      for (const m of moves) {
        m.piece.position.copy(m.to);
        m.piece.scale.copy(m.scale);
      }
      this.settleMotion = null;
      this.invalidate();
    };
    const tick = () => {
      const t = Math.min(1, (performance.now() - start) / 650),
        eased = t * t * (3 - 2 * t);
      for (const m of moves) {
        if (m.from) m.piece.position.lerpVectors(m.from, m.to, eased);
      }
      this.invalidate();
      if (t < 1) this.motionFrame = requestAnimationFrame(tick);
      else this.settleMotion?.();
    };
    tick();
  }
  private hoverRing: THREE.Mesh;
  private hovered: string | null = null;
  private tileTooltip: TileTooltip;
  private referenceRing: THREE.Mesh;
  private referenceLabel: HTMLDivElement;
  private pinnedReference: string | null = null;
  private selectable: Set<string> | null = null;
  private pending = false;
  private replaying = false;
  private gesture = new MapGesture();
  private raycaster = new THREE.Raycaster();
  private resize: ResizeObserver;
  private panelObserver: MutationObserver;
  private interactionPanel: HTMLElement | null = null;
  private interactionPositions: string[] = [];
  private interactionSignature = '';
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
  private strategyMap = false;
  private homeAtBottom = false;
  private strategySeat?: number;
  private strategyHome?: string;
  private strategyGame?: string;
  private boardInteraction = false;
  private decisionPositions: string[] = [];
  private materials = new Set<THREE.Material>();
  private geometries = new Set<THREE.BufferGeometry>();
  private textures = new Set<THREE.Texture>();
  private civilizationFlags = new CivilizationFlags(() => this.invalidate());
  private labelPositions: {
    position: string;
    at: THREE.Vector3;
    node: HTMLButtonElement;
    kind: 'city' | 'units' | 'destination' | 'collection' | 'strategy';
    offsetX?: number;
    offsetY?: number;
    ownershipBounds?: THREE.Vector3[];
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
    this.resourceOverlay = new ResourceOverlay(host);
    this.tileTooltip = new TileTooltip(host);
    this.renderer.domElement.setAttribute('aria-describedby', this.tileTooltip.node.id);
    this.scene.add(this.explorationOverlay.group);
    this.explorationLabel = document.createElement('div');
    this.explorationLabel.className = 'map-exploration-label';
    this.explorationLabel.textContent = 'Exploring';
    this.explorationLabel.setAttribute('role', 'status');
    this.explorationLabel.hidden = true;
    host.append(this.explorationLabel);
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
    this.scene.add(this.board, this.rings, this.placement.group);
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
    this.scene.add(this.combatOverlay.group);
    this.combatLabel = document.createElement('div');
    this.combatLabel.className = 'map-combat-label';
    this.combatLabel.hidden = true;
    this.combatLabel.setAttribute('role', 'status');
    this.combatIcon = mount(Swords, { target: this.combatLabel, props: { size: 14 } });
    this.combatLabelText = document.createTextNode('');
    this.combatLabel.append(this.combatLabelText);
    host.append(this.combatLabel);
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
    let viewportWidth = 0,
      viewportHeight = 0;
    this.resize = new ResizeObserver(() => {
      const { width, height } = host.getBoundingClientRect();
      if (!width || !height) return;
      const resized = viewportWidth !== width || viewportHeight !== height;
      viewportWidth = width;
      viewportHeight = height;
      this.renderer.setSize(width, height);
      this.camera.aspect = width / height;
      this.updateViewport();
      this.controls.enableZoom = true;
      this.renderer.domElement.style.touchAction = 'none';
      if (resized) this.reset();
    });
    this.resize.observe(host);
    const layout = host.closest('.play-layout')!;
    this.panelObserver = new MutationObserver(() => {
      const panel = layout.querySelector<HTMLElement>(
        '.board-collection, .board-movement, .board-decision, .board-ability, .tactics-decision, .exploration-panel, .board-context',
      );
      if (panel === this.interactionPanel) return;
      if (this.interactionPanel) this.resize.unobserve(this.interactionPanel);
      this.interactionPanel = panel;
      if (panel) this.resize.observe(panel);
      this.updateViewport();
    });
    this.panelObserver.observe(layout, { childList: true, subtree: true });
  }
  private compactMap() {
    return this.host.clientWidth <= 760 || (this.host.clientWidth <= 1000 && this.host.clientHeight <= 500);
  }
  private updateViewport() {
    const width = this.host.clientWidth,
      height = this.host.clientHeight;
    if (!width || !height) return;
    if (this.boardInteraction && this.compactMap() && this.interactionPanel?.getClientRects().length) {
      const board = this.host.getBoundingClientRect();
      const panel = this.interactionPanel.getBoundingClientRect();
      // Leave room for the map tools, then frame the usable area beside or above the panel.
      const landscape =
        width >= 600 && height <= 500 && !this.interactionPanel.classList.contains('selection-tray');
      const left = landscape ? Math.max(8, panel.right - board.left + 12) : 12;
      const right = width - 12;
      const top = landscape ? 60 : 12;
      const bottom = landscape ? height - 12 : Math.max(top + 80, panel.top - board.top - 12);
      this.camera.setViewOffset(
        width,
        height,
        width / 2 - (left + right) / 2,
        height / 2 - (top + bottom) / 2,
        width,
        height,
      );
    } else this.camera.clearViewOffset();
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
      if (typeof object.userData.position === 'string') {
        const { cityPosition, position: waterPosition } = object.userData;
        const inspectPort =
          cityPosition &&
          ((this.selectable === null && !this.seaGuide) ||
            (this.selectable?.has(cityPosition) && !this.selectable.has(waterPosition)));
        return {
          position: inspectPort ? cityPosition : waterPosition,
          pick: {
            kind: cityPosition && !inspectPort ? 'tile' : (object.userData.kind ?? 'tile'),
            player: object.userData.player,
            unit: object.userData.unit,
          },
        };
      }
      object = object.parent;
    }
    return null;
  }
  private setHovered(position: string | null, audible = false) {
    const changed = this.hovered !== position;
    if (audible && changed && position && this.canPick(position) && !this.gesture.dragging) this.hover();
    this.hovered = position;
    if (changed || audible || this.gesture.dragging)
      this.tileTooltip.hover(this.gesture.dragging ? null : position);
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
      : this.replaying
        ? 'grab'
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
  private bindTileHover(node: HTMLButtonElement, position: string) {
    node.setAttribute('aria-describedby', this.tileTooltip.node.id);
    node.onpointerenter = (event) => {
      if (event.pointerType === 'touch') return;
      this.tileTooltip.anchor(event.clientX, event.clientY);
      this.setHovered(position, true);
    };
    node.onpointerleave = () => this.setHovered(null);
    node.onfocus = () => {
      if (!node.matches(':focus-visible')) return;
      const rect = node.getBoundingClientRect();
      this.tileTooltip.anchor(rect.right, rect.top);
      this.setHovered(position);
      this.tileTooltip.hover(position);
    };
    node.onblur = () => this.setHovered(null);
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
      this.resourceOverlay.project(this.camera, w, h);
      for (const label of this.labelPositions) {
        const v = label.at.clone().project(this.camera);
        let x = ((v.x + 1) * w) / 2 + (label.offsetX ?? 0);
        const y = ((-v.y + 1) * h) / 2 + (label.offsetY ?? 0);
        if (label.ownershipBounds) {
          const flagLeft = Math.min(
            ...label.ownershipBounds.map((point) => ((point.clone().project(this.camera).x + 1) * w) / 2),
          );
          // Screen-space clearance keeps the mood clear of flags at every zoom and rotation.
          x = Math.min(x - 14, flagLeft - (this.topDown ? 9 : 11) - 4);
        }
        const edge = label.at
          .clone()
          .add(new THREE.Vector3(1.5, 0, 0))
          .project(this.camera);
        const scale =
          label.kind === 'strategy'
            ? Math.min(1.4, Math.hypot((edge.x - v.x) * w, (edge.y - v.y) * h) / 2 / 96)
            : 1;
        label.node.style.transform = `translate(-50%, -50%) translate(${x}px,${y}px) scale(${scale})`;
        label.node.style.display = v.z > 1 || v.z < -1 ? 'none' : '';
        if (label.kind === 'strategy') label.node.style.visibility = 'visible';
      }
      if (this.referenceRing.visible) {
        const v = this.referenceRing.position.clone().project(this.camera);
        this.referenceLabel.style.transform = `translate(-50%, 8px) translate(${((v.x + 1) * w) / 2}px,${((-v.y + 1) * h) / 2}px)`;
        this.referenceLabel.hidden = v.z > 1 || this.labelHost.matches('.choosing-pieces, .choosing-ability');
      }
      if (this.combatOverlay.group.children.length) {
        const v = this.combatOverlay.position.clone().project(this.camera);
        this.combatLabel.style.transform = `translate(-50%, 12px) translate(${((v.x + 1) * w) / 2}px,${((-v.y + 1) * h) / 2}px)`;
        this.combatLabel.hidden = v.z > 1 || v.z < -1;
      }
      if (this.explorationPositions.length) {
        const v = this.explorationOverlay.position.clone().project(this.camera);
        this.explorationLabel.style.transform = `translate(-50%, 8px) translate(${((v.x + 1) * w) / 2}px,${((-v.y + 1) * h) / 2}px)`;
        this.explorationLabel.hidden = v.z > 1 || v.z < -1;
      }
    });
  }
  highlightCoordinate(position: string | null) {
    const target = position ?? this.pinnedReference;
    const visible = !!target && this.tiles.has(target);
    this.referenceRing.visible = visible;
    this.referenceLabel.hidden = !visible || this.labelHost.matches('.choosing-pieces, .choosing-ability');
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
  locateExploration() {
    if (!this.explorationPositions.length) return;
    const points = this.explorationPositions.map(positionXY);
    const target = new THREE.Vector3(
      points.reduce((n, p) => n + p[0], 0) / points.length,
      0,
      points.reduce((n, p) => n + p[1], 0) / points.length,
    );
    this.camera.position.add(target.clone().sub(this.controls.target));
    this.controls.target.copy(target);
    this.controls.update();
    this.invalidate();
  }
  clearCoordinate() {
    this.pinnedReference = null;
    this.highlightCoordinate(null);
  }
  /** Capture only rendered terrain and pieces; no DOM panels or player information. */
  thumbnail(width: number, height: number): HTMLCanvasElement | null {
    if (!this.tiles.size || this.disposed) return null;
    this.settleMotion?.();
    const bounds = new THREE.Box3().setFromObject(this.board);
    if (bounds.isEmpty()) return null;
    const parts = this.board.children
      .filter((child) => child.visible)
      .map((child) => new THREE.Box3().setFromObject(child))
      .filter((box) => !box.isEmpty());
    const camera = thumbnailCamera(bounds, width, height, parts);
    const size = this.renderer.getSize(new THREE.Vector2());
    const ratio = this.renderer.getPixelRatio();
    const overlays = [
      this.rings,
      this.hoverRing,
      this.referenceRing,
      this.seaOverlay.group,
      this.combatOverlay.group,
      this.explorationOverlay.group,
      this.placement.group,
    ];
    const visibility = overlays.map((object) => object.visible);
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext('2d');
    if (!context) return null;
    try {
      overlays.forEach((object) => {
        object.visible = false;
      });
      this.renderer.setPixelRatio(1);
      this.renderer.setSize(width, height, false);
      this.renderer.render(this.scene, camera);
      context.drawImage(this.renderer.domElement, 0, 0);
    } finally {
      overlays.forEach((object, index) => {
        object.visible = visibility[index];
      });
      this.renderer.setPixelRatio(ratio);
      this.renderer.setSize(size.x, size.y, false);
      this.invalidate();
    }
    return canvas;
  }
  zoom(factor: number) {
    const direction = this.camera.position.clone().sub(this.controls.target);
    direction.multiplyScalar(factor).clampLength(this.controls.minDistance, this.controls.maxDistance);
    this.camera.position.copy(this.controls.target).add(direction);
    this.controls.update();
    this.invalidate();
  }
  reset() {
    this.clearCoordinate();
    this.renderer.domElement.setAttribute(
      'aria-label',
      `${this.strategyMap ? 'Strategy' : this.topDown ? 'Overhead' : '3D'} civilization map. Select a city or terrain tile. Drag to ${this.topDown ? 'pan' : 'orbit'}; scroll to zoom.`,
    );
    this.labelHost.classList.toggle('top-down', this.topDown);
    this.controls.enableRotate = !this.topDown;
    this.controls.minPolarAngle = this.topDown ? 0 : 0.08;
    this.controls.maxPolarAngle = this.topDown ? 0 : Math.PI * 0.43;
    this.controls.touches.ONE = this.topDown ? THREE.TOUCH.PAN : THREE.TOUCH.ROTATE;
    this.controls.mouseButtons.LEFT = this.topDown ? THREE.MOUSE.PAN : THREE.MOUSE.ROTATE;
    this.controls.maxDistance = 55;
    this.camera.far = 120;
    if (this.scene.fog instanceof THREE.Fog) {
      this.scene.fog.near = 55;
      this.scene.fog.far = 105;
    }
    this.controls.target.copy(this.center);
    const homeAngle =
      this.strategyHome && this.tiles.size
        ? strategyFrame([...this.tiles.keys()], this.strategyHome, this.camera.aspect).angle
        : undefined;
    this.camera.position
      .copy(this.center)
      .add(
        new THREE.Vector3(
          homeAngle === undefined
            ? this.topDown
              ? 0
              : 12.5
            : Math.sin(homeAngle) * (this.topDown ? 0.01 : Math.hypot(12.5, 17)),
          this.topDown ? 27 : 22,
          homeAngle === undefined
            ? this.topDown
              ? 0.01
              : 17
            : Math.cos(homeAngle) * (this.topDown ? 0.01 : Math.hypot(12.5, 17)),
        ).multiplyScalar(
          Math.max(1, 0.92 / this.camera.aspect) *
            (this.camera.aspect > 1.4 ? 0.85 : 1) *
            (this.seaGuide ? 1.15 : 1),
        ),
      );
    if (this.strategyMap && this.tiles.size) {
      const { angle, center, distance } = strategyFrame(
        [...this.tiles.keys()],
        this.strategyHome,
        this.camera.aspect,
      );
      this.controls.maxDistance = Math.max(55, distance * 1.3);
      this.camera.far = Math.max(120, this.controls.maxDistance + 20);
      if (this.scene.fog instanceof THREE.Fog) {
        this.scene.fog.near = this.controls.maxDistance + 10;
        this.scene.fog.far = this.controls.maxDistance + 60;
      }
      this.controls.target.set(center[0], 0, center[1]);
      this.camera.position
        .copy(this.controls.target)
        .add(new THREE.Vector3(Math.sin(angle) * 0.01, distance, Math.cos(angle) * 0.01));
    }
    this.camera.updateProjectionMatrix();
    this.controls.update();
    this.invalidate();
  }
  private centerInteraction() {
    const positions = this.decisionPositions.length
      ? this.decisionPositions
      : this.compactMap()
        ? this.interactionPositions
        : [];
    if (!positions.length || !this.host.clientWidth || !this.host.clientHeight) return;
    const points = positions.map(positionXY);
    const xs = points.map(([x]) => x),
      zs = points.map(([, z]) => z);
    const target = new THREE.Vector3(
      (Math.min(...xs) + Math.max(...xs)) / 2,
      0,
      (Math.min(...zs) + Math.max(...zs)) / 2,
    );
    // Opening a selection may pan to its targets, but must keep the player's zoom and angle.
    this.camera.position.add(target.clone().sub(this.controls.target));
    this.controls.target.copy(target);
    this.controls.update();
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
    group.position.set(x, 0, z);
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
        rock.position.set((i - 1) * 0.32, height / 2, (i % 2) * 0.24 - 0.1);
        rock.rotation.y = i;
        group.add(rock);
        const cap = this.mesh(new THREE.ConeGeometry(0.14, height * 0.28, 5), this.material('#eee9d7'));
        cap.position.copy(rock.position);
        cap.position.y = height * 0.87;
        cap.rotation.y = i;
        group.add(cap);
      }
    }
    if (kind === 'Fertile') {
      const fieldMat = this.material('#c6b968');
      for (let i = 0; i < 5; i++) {
        const furrow = this.mesh(new THREE.BoxGeometry(0.06, 0.035, 0.7), fieldMat);
        furrow.position.set(-0.35 + i * 0.14, 0.0175, 0.03);
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
        rock.geometry.computeBoundingBox();
        rock.position.y = -rock.geometry.boundingBox!.min.y * rock.scale.y;
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
  private ownershipBadge(index: number, symbols?: string[], barbarian = false) {
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
    if (barbarian) {
      context.save();
      context.scale(64 / 24, 64 / 24);
      context.lineWidth = 1.8;
      context.lineCap = 'round';
      context.lineJoin = 'round';
      context.stroke(
        new Path2D(
          'M6 11C3 10 2 7 3 3l4 4M18 11c3-1 4-4 3-8l-4 4M6 18v-6a6 6 0 0 1 12 0v6M5 18h14M12 7v11M8 18v3m8-3v3',
        ),
      );
      context.restore();
    } else context.fillText(playerSymbol(index, symbols), 32, 33);
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
    this.settleMotion?.();
    this.placement.stop();
    this.clearCollectionBadges();
    for (const badge of this.unitBadges) void unmount(badge);
    this.unitBadges = [];
    this.board.traverse((o) => {
      if (o instanceof THREE.Mesh || o instanceof THREE.Sprite) {
        if (o instanceof THREE.Mesh || o instanceof THREE.Line) {
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
    this.buildings.clear();
    this.labelHost.replaceChildren();
    this.labelPositions = [];
    this.moveMarkerSignature = '';
  }
  update(s: Session) {
    if (!s.game) return;
    // Keep the viewer's home orientation when replay temporarily clears the interactive seat.
    const home = s.homeAtBottom ? strategyHome(s.game, s.seat) : undefined;
    const homePreferenceChanged = this.homeAtBottom !== s.homeAtBottom;
    const orientationChanged =
      homePreferenceChanged ||
      this.strategySeat !== s.seat ||
      this.strategyGame !== s.game.board_history?.id ||
      (!this.strategyHome && !!home);
    if (orientationChanged) {
      this.homeAtBottom = s.homeAtBottom;
      this.strategySeat = s.seat;
      this.strategyGame = s.game.board_history?.id;
      this.strategyHome = home;
    }
    const playback = s.playback;
    const history = s.game.board_history;
    const resourceFrame = playback?.frame ?? history?.frames.at(-1);
    const historyId = history?.id ?? '';
    if (!resourceFrame) {
      this.resourceOverlay.clear();
      this.resourceCursor = publicActions(s.game).length;
      this.resourceHistory = '';
      this.resourcePlayback = false;
    }
    if (
      resourceFrame &&
      (resourceFrame.cursor !== this.resourceCursor ||
        !!playback !== this.resourcePlayback ||
        historyId !== this.resourceHistory)
    ) {
      const sameHistory =
        historyId === this.resourceHistory ||
        (!this.resourceHistory && history?.frames[0]?.cursor === this.resourceCursor);
      const from = playback ? history?.frames[playback.index - 1]?.cursor : this.resourceCursor;
      const show = playback
        ? playback.index > playback.start
        : sameHistory &&
          !this.resourcePlayback &&
          this.resourceCursor >= 0 &&
          resourceFrame.cursor > this.resourceCursor;
      this.resourceOverlay.show(
        show && from !== undefined ? frameResources(s.game, from, resourceFrame).markers : [],
        s,
        !s.reducedMotion && sameHistory && resourceFrame.cursor > this.resourceCursor,
      );
      this.resourceCursor = resourceFrame.cursor;
      this.resourceHistory = historyId;
      this.resourcePlayback = !!playback;
    }
    const battle = s.battles?.[0];
    const replayPositions =
      playback && playback.index > playback.start
        ? frameDetails(s.game.board_history?.frames[playback.index - 1], playback.frame, s.game).positions
        : [];
    const cursor = playback
      ? `replay:${playback.frame?.cursor}`
      : `live:${s.game.board_history?.frames.at(-1)?.cursor ?? s.game.log_index}`;
    const animate =
      !s.reducedMotion &&
      !!this.lastBoardCursor &&
      cursor !== this.lastBoardCursor &&
      (playback ? playback.animate : !this.wasPlayback);
    const boardFrame = playback?.frame?.cursor ?? history?.frames.at(-1)?.cursor ?? s.game.log_index;
    const placeForward =
      animate &&
      boardFrame > this.lastBoardFrame &&
      historyId === this.lastBoardHistory &&
      !!playback === this.wasPlayback;
    if (!animate && (s.reducedMotion || !!playback !== this.wasPlayback)) {
      this.settleMotion?.();
      this.placement.stop();
    }
    this.lastBoardCursor = cursor;
    this.lastBoardFrame = boardFrame;
    this.lastBoardHistory = historyId;
    this.wasPlayback = !!playback;
    if (playback) {
      const frame = playback.frame;
      s = {
        ...s,
        game: frame
          ? {
              ...s.game,
              state: 'Playing',
              events: [],
              age: frame.age,
              round: frame.round,
              map: { tiles: frame.tiles },
              players: frame.players,
            }
          : s.game,
        view: null,
        seat: undefined,
        pending: true,
        mode: 'overview',
        focus: null,
        city: null,
        tilePanel: false,
        seaRoutes: false,
        pirateSpawns: false,
        seaRouteStart: null,
        selectedUnits: [],
        moveDestinations: [],
        landingTargets: [],
        selection: [],
        decisionSelection: [],
        abilitiesOpen: false,
      };
    }
    if (!s.game) return;
    const mapChoices = mapDecisionOptions(s.view?.decision);
    this.tileTooltip.update(s);
    const decisionPositions = [...new Set(mapChoices.map((o) => o.position!))];
    const pieceDecision = mapChoices.some((o) => o.mapTarget);
    this.decisionPositions = decisionPositions;
    const ability = activeCityAbility(s);
    const moodPositions = s.mode === 'happiness' ? happinessCities(s) : [];
    const combat = battle?.location ?? (playback ? (playback.frame?.combat ?? null) : activeCombat(s.game));
    const exploration = s.view?.explorationDecision;
    this.explorationPositions = exploration?.choices[0]?.tiles.map(([p]) => p) ?? explorationPreview(s);
    this.explorationOverlay.update(this.explorationPositions);
    this.explorationLabel.textContent = exploration ? 'Exploring' : 'Will reveal';
    this.explorationLabel.hidden = !this.explorationPositions.length;
    this.explorationLabel.dataset.positions = this.explorationPositions.join(' ');
    const abilityPositions = [...new Set(ability?.offers.map((offer) => offer.position!) ?? [])];
    this.interactionPositions = exploration
      ? this.explorationPositions
      : s.view?.decision?.tacticsSelection && combat
        ? [combat.attacker.position, combat.defender.position]
        : ability
          ? abilityPositions
          : s.mode === 'happiness'
            ? moodPositions
            : s.mode === 'collect'
              ? [
                  ...new Set(
                    [
                      s.city!,
                      ...(s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ??
                        []),
                    ].filter(Boolean),
                  ),
                ]
              : s.mode === 'settlers'
                ? [
                    ...new Set([
                      ...(s.unitPosition ? [s.unitPosition] : []),
                      ...(s.view?.units
                        ?.filter((u) => s.selectedUnits.includes(u.id))
                        .map((u) => u.position) ?? []),
                      ...s.moveDestinations.map((d) => d.position),
                      ...this.explorationPositions,
                      ...(s.landingTargets ?? []),
                    ]),
                  ]
                : s.tilePanel && s.focus
                  ? [s.focus]
                  : [];
    this.labelHost.classList.toggle('collecting', s.mode === 'collect');
    this.labelHost.classList.toggle('choosing-happiness', s.mode === 'happiness');
    const decisionSelected = s.decisionSelection.flatMap((i) => mapChoices[i]?.position ?? []);
    const interacting =
      s.mode === 'collect' ||
      s.mode === 'happiness' ||
      s.mode === 'settlers' ||
      s.tilePanel ||
      mapChoices.length > 0 ||
      !!ability ||
      !!exploration ||
      !!s.view?.decision?.tacticsSelection;
    if (interacting !== this.boardInteraction) {
      this.boardInteraction = interacting;
      this.updateViewport();
    }
    const seaGuide = s.seaRoutes && s.mode === 'overview' && !s.view?.decision;
    const guideChanged = seaGuide !== this.seaGuide;
    this.seaGuide = seaGuide;
    this.seaPreviewAllowed =
      s.mode === 'overview' &&
      !ability &&
      !s.pending &&
      !s.view?.decision &&
      !s.view?.explorationDecision &&
      !s.view?.choiceDecision &&
      !s.view?.objectiveDecision;
    this.seaRouteStart = s.seaRouteStart;
    this.seaOverlay.update(s.game.map.tiles, s.view?.seaRoutes ?? []);
    this.combatOverlay.update(combat);
    if (s.reducedMotion || !battle) this.combatOverlay.stopAnimation();
    if (battle?.key !== this.battleKey) {
      this.battleKey = battle?.key ?? '';
      if (battle && combat && s.battleAnimate && !s.reducedMotion)
        this.combatOverlay.pulse(() => this.invalidate());
      if (battle?.location && playback && !s.reducedMotion) {
        const [x, z] = positionXY(battle.location.defender.position);
        const target = new THREE.Vector3(x, 0, z);
        this.camera.position.add(target.clone().sub(this.controls.target));
        this.controls.target.copy(target);
        this.controls.update();
      }
    }
    this.combatLabel.hidden = !combat;
    if (combat) {
      this.combatLabelText.textContent = battle?.combat.result ?? `Battle · Round ${combat.round}`;
      this.combatLabel.setAttribute(
        'aria-label',
        `${s.game.players[combat.attacker.player]?.civilization ?? 'Attacker'} attacks ${s.game.players[combat.defender.player]?.civilization ?? 'defender'} · ${combat.attacker.position} to ${combat.defender.position} · Round ${combat.round}`,
      );
    }
    this.pending = s.pending;
    this.replaying = !!s.playback;
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
      : s.view?.decision
        ? new Set(decisionPositions)
        : s.mode === 'happiness'
          ? new Set(moodPositions)
          : s.mode === 'collect'
            ? new Set([
                ...(s.view?.cities.map((c) => c.position) ?? []),
                ...(s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ?? []),
              ])
            : s.mode === 'settlers'
              ? new Set([
                  ...(s.view?.units?.map((u) => u.position) ?? []),
                  ...(s.view?.nomadCities ?? []),
                  ...s.moveDestinations.map((d) => d.position),
                  ...(s.landingTargets ?? []),
                ])
              : ability
                ? new Set(abilityPositions)
                : null;
    const signature = JSON.stringify([
      mapTiles,
      s.game.players.map((p) => [p.cities, p.units]),
      s.view?.players,
      s.seat,
      s.colorBlind,
      s.playerColors,
      s.playerSymbols,
      s.strategyMap,
    ]);
    if (signature !== this.lastSignature) {
      this.lastSignature = signature;
      this.settleMotion?.();
      this.placement.stop();
      const previous = new Map(this.pieces.map((p) => [this.pieceKey(p), p.position.clone()]));
      const previousBuildings = new Set(this.buildings.keys());
      const previousCities = new Set(
        this.pieces
          .filter((p) => p.userData.kind === 'city' && !p.userData.building)
          .map((p) => p.userData.position as string),
      );
      const hadTiles = this.tiles.size > 0;
      this.clearBoard();
      const models = new PieceModels(
        (color) => this.material(color),
        (geo, mat) => this.mesh(geo, mat),
      );
      const cityPositions = new Set(s.game.players.flatMap((p) => p.cities?.map((c) => c.position) ?? []));
      const unitPositions = new Set(s.game.players.flatMap((p) => p.units?.map((u) => u.position) ?? []));
      const coords = s.game.map.tiles.map(([p]) => positionXY(p));
      // The civilization draft has no map yet. Empty bounds would poison the camera with NaN.
      if (coords.length) {
        const minX = Math.min(...coords.map((c) => c[0])),
          maxX = Math.max(...coords.map((c) => c[0])),
          minZ = Math.min(...coords.map((c) => c[1])),
          maxZ = Math.max(...coords.map((c) => c[1]));
        this.center.set((minX + maxX) / 2, 0, (minZ + maxZ) / 2);
      }
      for (const [position, terrain] of mapTiles) {
        const exhausted = typeof terrain !== 'string';
        const kind = exhausted ? terrain.Exhausted : terrain;
        const [x, z] = positionXY(position);
        const height = kind === 'Water' ? 0.13 : kind === 'Unexplored' ? 0.27 : 0.43;
        const group = new THREE.Group();
        group.position.set(x, height / 2 - 0.12, z);
        const top = this.material(
          exhausted
            ? `#${new THREE.Color(terrainColor[kind]).lerp(new THREE.Color('#8d7c7c'), 0.6).getHexString()}`
            : terrainColor[kind],
        );
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
        if (exhausted) {
          // Hatching distinguishes depletion from barren ground without implying normal terrain restrictions.
          const hatch = this.material('#5f5052');
          for (let i = -1; i <= 1; i++) {
            const line = this.mesh(new THREE.BoxGeometry(i === 0 ? 1.5 : 1.05, 0.016, 0.045), hatch);
            line.position.set(i * 0.25, 0.03, i * 0.25);
            line.rotation.y = Math.PI / 4;
            terrainGroup.add(line);
          }
        }
        // Keep the foreground clear for pieces instead of burying them in trees/peaks.
        if (!exhausted && !s.strategyMap && !cityPositions.has(position)) {
          this.addTerrain(terrainGroup, kind, position.charCodeAt(0));
          if (unitPositions.has(position) && (kind === 'Forest' || kind === 'Mountain')) {
            terrainGroup.scale.set(0.8, 0.65, 0.6);
            terrainGroup.position.z = -0.34;
          }
        }
        group.add(terrainGroup);
        this.board.add(group);
      }
      if (!s.strategyMap) {
        const portSites = new Map(
          s.game.players.flatMap((player) =>
            (player.cities ?? []).flatMap((city) => {
              if (
                city.city_pieces?.port == null ||
                !city.port_position ||
                !mapTiles.some(
                  ([position, terrain]) => position === city.port_position && terrain === 'Water',
                )
              )
                return [];
              const placement = portPlacement(city.position, city.port_position);
              return placement ? [[city.position, { ...placement, water: city.port_position }] as const] : [];
            }),
          ),
        );
        const harborDirections = new Map([...portSites.values()].map((port) => [port.water, port.rotation]));
        const unitStacks = new Map<string, number>();
        const unitsByTile = new Map<string, number>();
        for (const player of s.game.players)
          for (const unit of player.units ?? [])
            unitsByTile.set(unit.position, (unitsByTile.get(unit.position) ?? 0) + 1);
        for (const player of s.game.players) {
          for (const city of player.cities ?? []) {
            const [x, z] = positionXY(city.position);
            const cityModel = new THREE.Group();
            const ownerColor = playerColor(player.id, s.colorBlind, s.playerColors);
            const wonders = (city.city_pieces?.wonders ?? []).filter((name) => name !== 'Hidden');
            const additions = Object.entries(city.city_pieces ?? {}).filter(
              (entry): entry is [BuildingKind, number] =>
                entry[0] !== 'wonders' &&
                typeof entry[1] === 'number' &&
                !(entry[0] === 'port' && portSites.has(city.position)),
            );
            const settlement = models.settlement(ownerColor, player.civilization);
            settlement.scale.setScalar(additions.length ? 0.57 : 0.88);
            settlement.position.set(additions.length === 1 ? -0.2 : 0, 0, -0.09);
            cityModel.add(settlement);
            const pole = this.mesh(
              new THREE.CylinderGeometry(0.018, 0.024, 1.06, 6),
              this.material('#624d30'),
            );
            pole.position.set(0.12, 0.54, -0.12);
            cityModel.add(pole);
            const flagEdge = this.material(ownerColor);
            const flagFace = new THREE.MeshBasicMaterial({
              map: this.civilizationFlags.texture(player.civilization, ownerColor),
              toneMapped: false,
            });
            this.materials.add(flagFace);
            const flag = this.mesh(new THREE.BoxGeometry(0.45, 0.3, 0.025), [
              flagEdge,
              flagEdge,
              flagEdge,
              flagEdge,
              flagFace,
              flagFace,
            ]);
            flag.position.set(0.345, 0.93, -0.12);
            cityModel.add(flag);
            const ownershipPieces: THREE.Object3D[] = [pole, flag];
            cityModel.position.set(x, 0.315, z);
            cityModel.rotation.y = 0.25;
            const capital = s.view?.players.find((p) => p.index === player.id)?.capital === city.position;
            if (capital) {
              const gold = this.material('#d5af55');
              const band = this.mesh(new THREE.CylinderGeometry(0.16, 0.16, 0.07, 12), gold);
              band.position.set(0.12, 1.12, -0.12);
              cityModel.add(band);
              ownershipPieces.push(band);
              for (let i = 0; i < 3; i++) {
                const point = this.mesh(new THREE.ConeGeometry(0.055, 0.15, 4), gold);
                point.position.set(0.12 + (i - 1) * 0.11, 1.22, -0.12);
                cityModel.add(point);
                ownershipPieces.push(point);
              }
            }
            cityModel.userData = { position: city.position, kind: 'city', player: player.id };
            this.pieces.push(cityModel);
            let ownershipBadge: THREE.Sprite | undefined;
            if (s.colorBlind) {
              const badge = this.ownershipBadge(
                player.id,
                s.playerSymbols,
                player.civilization === 'Barbarians',
              );
              badge.position.set(0.27, 1.27, -0.12);
              cityModel.add(badge);
              ownershipBadge = badge;
              ownershipPieces.push(badge);
            }
            const slots =
              additions.length === 1
                ? [[0.4, 0.18]]
                : additions.length === 2
                  ? [
                      [-0.45, 0.18],
                      [0.45, 0.18],
                    ]
                  : additions.length === 3
                    ? [
                        [0, -0.52],
                        [-0.43, 0.28],
                        [0.43, 0.28],
                      ]
                    : [
                        [-0.42, -0.37],
                        [0.42, -0.37],
                        [-0.42, 0.37],
                        [0.42, 0.37],
                      ];
            const heights: BuildingKind[] = [
              'obelisk',
              'observatory',
              'fortress',
              'temple',
              'port',
              'academy',
              'market',
            ];
            additions.sort(([a], [b]) => heights.indexOf(a) - heights.indexOf(b));
            const ordinary = [settlement];
            for (const [j, [name, buildingOwner]] of additions.entries()) {
              const annex = models.building(
                name,
                playerColor(buildingOwner, s.colorBlind, s.playerColors),
                player.civilization,
              );
              const [ax, az] = slots[j % slots.length];
              annex.scale.setScalar(additions.length < 3 ? 0.62 : 0.55);
              annex.position.set(ax, 0, az);
              cityModel.add(annex);
              this.buildings.set(`${city.position}:building:${name}`, annex);
              ordinary.push(annex);
            }
            if (wonders.length) {
              // Reserve the rear of the hex for landmarks; keep ordinary buildings,
              // the settlement and ownership flag visible in the foreground.
              ordinary.forEach((piece, index) => {
                const count = Math.min(3, ordinary.length),
                  row = Math.floor(index / 3);
                const rowCount = Math.min(3, ordinary.length - row * 3);
                piece.scale.setScalar(ordinary.length > 3 ? 0.36 : 0.46);
                piece.position.set(((index % count) - (rowCount - 1) / 2) * 0.43, 0, row ? 0.57 : 0.18);
              });
              wonders.forEach((name, index) => {
                const landmark = models.wonder(name, ownerColor);
                const columns = Math.min(3, wonders.length),
                  row = Math.floor(index / columns);
                const scale =
                  wonders.length === 1 ? 0.8 : wonders.length === 2 ? 0.57 : wonders.length <= 3 ? 0.4 : 0.28;
                landmark.scale.setScalar(scale);
                landmark.position.set(
                  ((index % columns) - (columns - 1) / 2) * scale * 1.08,
                  0,
                  wonders.length <= 3 ? -0.33 : -0.55 + row * 0.24,
                );
                landmark.userData.wonder = name;
                cityModel.add(landmark);
                this.buildings.set(`${city.position}:wonder:${name}`, landmark);
              });
              for (const marker of ownershipPieces) marker.position.x += 0.28;
            }
            const portSite = portSites.get(city.position);
            if (portSite && city.city_pieces?.port != null) {
              const dock = new THREE.Group();
              const port = models.building(
                'port',
                playerColor(city.city_pieces.port, s.colorBlind, s.playerColors),
                player.civilization,
              );
              port.scale.setScalar(0.5);
              dock.add(port);
              // A short sloping gangway connects the water-level wharf to its city.
              const gangway = this.mesh(new THREE.BoxGeometry(0.16, 0.04, 0.36), this.material('#a28250'));
              gangway.position.set(0, 0.19, -0.31);
              gangway.rotation.x = 0.53;
              dock.add(gangway);
              dock.position.set(portSite.x, 0.01, portSite.z);
              dock.rotation.y = portSite.rotation;
              dock.userData = {
                position: portSite.water,
                cityPosition: city.position,
                kind: 'city',
                player: player.id,
                building: 'port',
              };
              this.board.add(dock);
              this.pieces.push(dock);
              this.buildings.set(`${city.position}:building:port`, dock);
            }
            this.board.add(cityModel);
            cityModel.updateWorldMatrix(true, true);
            const ownership = new THREE.Box3().setFromObject(flag);
            if (ownershipBadge) {
              const center = ownershipBadge.getWorldPosition(new THREE.Vector3());
              // A sprite faces the camera; its enclosing cube stays safe as the camera turns.
              ownership.expandByPoint(center.clone().addScalar(0.3));
              ownership.expandByPoint(center.clone().addScalar(-0.3));
            }
            const label = document.createElement('button');
            label.className = 'city-map-label';
            label.style.setProperty('--player-color', playerColor(player.id, s.colorBlind, s.playerColors));
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
            name.textContent = `${s.colorBlind ? playerSymbol(player.id, s.playerSymbols) + ' ' : ''}${player.civilization}${capital ? ' ♛' : ''} · ${city.position} · ${mood}`;
            label.append(face, name);
            label.setAttribute('aria-label', `Select ${player.civilization} city ${city.position} · ${mood}`);
            label.onclick = () => {
              if (this.canPick(city.position)) this.pick(city.position, { kind: 'city', player: player.id });
            };
            this.bindTileHover(label, city.position);
            this.labelHost.append(label);
            this.labelPositions.push({
              position: city.position,
              at: new THREE.Vector3(x, 1.4, z),
              node: label,
              kind: 'city',
              ownershipBounds: [ownership.min.x, ownership.max.x].flatMap((x) =>
                [ownership.min.y, ownership.max.y].flatMap((y) =>
                  [ownership.min.z, ownership.max.z].map((z) => new THREE.Vector3(x, y, z)),
                ),
              ),
            });
          }
          for (const unit of player.units ?? []) {
            const [x, z] = positionXY(unit.position);
            const pawn = models.unit(
              unit.unit_type,
              playerColor(player.id, s.colorBlind, s.playerColors),
              unit.pirate,
              player.civilization,
            );
            pawn.userData = { position: unit.position, kind: 'unit', unit: unit.id, player: player.id };
            this.pieces.push(pawn);
            const stackIndex = unitStacks.get(unit.position) ?? 0;
            unitStacks.set(unit.position, stackIndex + 1);
            const tile = this.tiles.get(unit.position);
            const surface = tile
              ? tile.parent!.position.y + (tile.geometry as THREE.CylinderGeometry).parameters.height / 2
              : 0.31;
            const inCity = cityPositions.has(unit.position);
            const positions = inCity
              ? [
                  [-0.43, 0.65],
                  [0, 0.69],
                  [0.43, 0.65],
                  [-0.69, 0.03],
                  [0.69, 0.03],
                  [0, -0.68],
                ]
              : [
                  [-0.43, 0.4],
                  [0, 0.4],
                  [0.43, 0.4],
                  [-0.43, -0.12],
                  [0, -0.12],
                  [0.43, -0.12],
                ];
            const count = unitsByTile.get(unit.position)!;
            let [ux, uz] = positions[stackIndex % positions.length];
            if (count > positions.length) {
              // Large mixed stacks fan out instead of drawing several units in one slot.
              const angle = (stackIndex / count) * Math.PI * 2;
              ux = Math.sin(angle) * 0.72;
              uz = Math.cos(angle) * 0.66;
            } else if (!inCity && count === 1) {
              ux = 0;
            }
            const ship = unit.unit_type === 'Ship';
            if (ship && count > 1 && count <= 4) {
              ux = stackIndex % 2 ? 0.27 : -0.27;
              uz = stackIndex < 2 ? 0.36 : -0.36;
            }
            const harborDirection = ship ? harborDirections.get(unit.position) : undefined;
            const inHarbor = harborDirection !== undefined && count <= 4;
            if (inHarbor) {
              // Moor side by side facing away from the wharf, leaving its entrance clear.
              const across = (stackIndex - (count - 1) / 2) * (count > 2 ? 0.3 : 0.42);
              ux = across * Math.cos(harborDirection) + 0.08 * Math.sin(harborDirection);
              uz = -across * Math.sin(harborDirection) + 0.08 * Math.cos(harborDirection);
            }
            pawn.scale.setScalar(
              count > positions.length
                ? Math.min(0.65, 5.2 / count)
                : inHarbor
                  ? count === 1
                    ? 0.8
                    : count === 2
                      ? 0.64
                      : count === 3
                        ? 0.54
                        : 0.46
                  : ship
                    ? 0.88
                    : 0.85,
            );
            pawn.position.set(x + ux, ship ? surface : surface + 0.02, z + uz);
            pawn.rotation.y = inHarbor ? harborDirection : -0.22;
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
            label.style.setProperty('--player-color', playerColor(player.id, s.colorBlind, s.playerColors));
            label.setAttribute('aria-label', `Inspect ${description}`);
            this.unitBadges.push(
              mount(UnitMapBadge, {
                target: label,
                props: {
                  groups: [...counts.values()],
                  symbol:
                    s.colorBlind && !['Pirates', 'Barbarians'].includes(player.civilization)
                      ? playerSymbol(player.id, s.playerSymbols)
                      : '',
                  barbarian: s.colorBlind && player.civilization === 'Barbarians',
                  pirate: player.civilization === 'Pirates' || surfaceUnits.some((unit) => unit.pirate),
                },
              }),
            );
            label.onclick = () => {
              if (this.canPick(position)) this.pick(position, { kind: 'units', player: player.id });
            };
            this.bindTileHover(label, position);
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
      } else {
        for (const tile of strategyTiles({ map: { tiles: mapTiles }, players: s.game.players })) {
          const node = document.createElement('button');
          node.className = 'strategy-map-tile';
          node.style.visibility = 'hidden';
          node.dataset.position = tile.position;
          node.setAttribute('aria-label', strategyDescription(tile));
          node.onclick = () => {
            if (this.canPick(tile.position)) this.pick(tile.position);
          };
          this.bindTileHover(node, tile.position);
          this.unitBadges.push(
            mount(StrategyMapTile, {
              target: node,
              props: {
                tile,
                colorBlind: s.colorBlind,
                playerColors: s.playerColors ?? [],
                playerSymbols: s.playerSymbols ?? [],
              },
            }),
          );
          this.labelHost.append(node);
          const [x, z] = positionXY(tile.position);
          this.labelPositions.push({
            position: tile.position,
            at: new THREE.Vector3(x, 0.4, z),
            node,
            kind: 'strategy',
          });
        }
      }
      const constructing = placeForward
        ? [...this.buildings]
            .filter(([key]) => !previousBuildings.has(key) && previousCities.has(key.split(':')[0]))
            .map(([, model]) => model)
        : [];
      const appearing = placeForward
        ? [
            ...constructing.map((model) => ({ model, kind: 'building' as const })),
            ...this.pieces
              .filter((piece) =>
                piece.userData.kind === 'unit'
                  ? !previous.has(this.pieceKey(piece))
                  : !piece.userData.building && !previousCities.has(piece.userData.position),
              )
              .map((model) => ({
                model,
                kind: model.userData.kind === 'unit' ? ('unit' as const) : ('building' as const),
              })),
          ]
        : [];
      if (animate) this.animatePieces(previous, new Set(appearing.map(({ model }) => model)));
      this.placement.play(appearing, () => this.invalidate());
      // Frame a newly created map, including when the last opponent finishes choosing.
      if (!hadTiles && this.tiles.size > 0) this.reset();
    }
    const settler = s.view?.units?.find((u) => s.selectedUnits.includes(u.id));
    const focusedPosition =
      s.mode === 'settlers'
        ? (s.unitPosition ?? settler?.position)
        : s.mode === 'overview'
          ? (s.focus ?? s.city)
          : s.city;
    const selected = playback
      ? replayPositions
      : mapChoices.length
        ? [...decisionSelected, ...(pieceDecision ? [s.decisionPosition ?? decisionPositions[0]] : [])]
        : exploration
          ? exploration.destination
            ? [exploration.destination]
            : []
          : s.mode === 'happiness'
            ? Object.keys(s.happinessSteps ?? {})
            : s.mode === 'settlers'
              ? [s.unitPosition ?? settler?.position, s.moveTarget].filter((p): p is string => !!p)
              : s.mode === 'collect'
                ? s.selection.map((c) => c.position)
                : s.tilePanel && focusedPosition
                  ? [focusedPosition]
                  : ability && s.abilityCity
                    ? [s.abilityCity]
                    : [];
    const available = mapChoices.length
      ? decisionPositions
      : placement
        ? placement.tiles.map(([position]) => position)
        : s.mode === 'happiness'
          ? moodPositions
          : s.mode === 'settlers'
            ? [
                ...new Set([
                  ...(s.view?.units?.map((u) => u.position) ?? []),
                  ...s.moveDestinations.map((d) => d.position),
                  ...(s.landingTargets ?? []),
                ]),
              ]
            : s.mode === 'collect'
              ? (s.view?.cities.find((c) => c.position === s.city)?.choices.map((c) => c.position) ?? [])
              : abilityPositions;
    const pirateGuide =
      s.pirateSpawns &&
      s.mode === 'overview' &&
      !playback &&
      !s.view?.decision &&
      !s.view?.choiceDecision &&
      !s.view?.objectiveDecision &&
      !exploration &&
      !ability;
    const pirateSpawns = pirateGuide
      ? s.view?.pirateSpawns?.find(
          (p) => p.player === (s.pirateSpawnPlayer ?? s.seat ?? s.view?.activePlayer),
        )
      : undefined;
    const barb =
      pirateGuide && s.threatGuide === 'barbarians'
        ? s.view?.barbarianGuide?.find(
            (p) => p.player === (s.pirateSpawnPlayer ?? s.seat ?? s.view?.activePlayer),
          )
        : undefined;
    const barbarianSpawn = barb?.spawn ?? [];
    const barbarianReinforce = barb?.reinforce ?? [];
    const barbarianMoves = barb?.moves ?? [];
    const pirateFirst = s.threatGuide !== 'barbarians' ? (pirateSpawns?.first ?? []) : [];
    const pirateSecond = s.threatGuide !== 'barbarians' ? (pirateSpawns?.second ?? []) : [];
    const selectionSig = JSON.stringify([
      selected,
      available,
      pirateFirst,
      pirateSecond,
      barbarianSpawn,
      barbarianReinforce,
      barbarianMoves,
    ]);
    if (selectionSig !== this.selectionSignature) {
      this.selectionSignature = selectionSig;
      this.rings.traverse((o) => {
        if (o instanceof THREE.Mesh || o instanceof THREE.Line) {
          o.geometry.dispose();
          this.geometries.delete(o.geometry);
          (o.material as THREE.Material).dispose();
          this.materials.delete(o.material as THREE.Material);
        }
      });
      this.rings.clear();
      for (const pos of new Set([
        ...selected,
        ...available,
        ...pirateSecond,
        ...barbarianSpawn,
        ...barbarianReinforce,
      ])) {
        const [x, z] = positionXY(pos);
        const ring = this.mesh(
          new THREE.TorusGeometry(
            0.99,
            selected.includes(pos) || pirateSecond.includes(pos) ? 0.05 : 0.025,
            6,
            6,
          ),
          new THREE.MeshBasicMaterial({
            color: barbarianSpawn.includes(pos)
              ? '#edaa4d'
              : barbarianReinforce.includes(pos)
                ? '#b789d8'
                : pirateFirst.includes(pos)
                  ? '#e96a55'
                  : pirateSecond.includes(pos)
                    ? '#f7c65b'
                    : selected.includes(pos)
                      ? '#ffd16b'
                      : '#ebdab3',
          }),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.position.set(x, selected.includes(pos) ? 0.48 : 0.35, z);
        this.materials.add(ring.material as THREE.Material);
        this.rings.add(ring);
        if (selected.includes(pos)) {
          const material = new THREE.MeshBasicMaterial({
            color: '#ffc450',
            transparent: true,
            opacity: 0.14,
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

      for (const { from, to } of barbarianMoves) {
        const [x, z] = positionXY(from),
          [tx, tz] = positionXY(to);
        const material = new THREE.LineDashedMaterial({
          color: '#e96a55',
          dashSize: 0.14,
          gapSize: 0.08,
          depthTest: false,
        });
        const geometry = new THREE.BufferGeometry().setFromPoints([
          new THREE.Vector3(x, 0.5, z),
          new THREE.Vector3(tx, 0.5, tz),
        ]);
        this.geometries.add(geometry);
        this.materials.add(material);
        const line = new THREE.Line(geometry, material);
        line.computeLineDistances();
        line.renderOrder = 3;
        this.rings.add(line);
        const dx = tx - x,
          dz = tz - z,
          length = Math.hypot(dx, dz),
          ux = dx / length,
          uz = dz / length;
        const ax = x + dx * 0.68,
          az = z + dz * 0.68;
        const arrowGeo = new THREE.BufferGeometry().setAttribute(
          'position',
          new THREE.Float32BufferAttribute(
            [
              ax + ux * 0.18,
              0.51,
              az + uz * 0.18,
              ax - ux * 0.1 - uz * 0.12,
              0.51,
              az - uz * 0.1 + ux * 0.12,
              ax - ux * 0.1 + uz * 0.12,
              0.51,
              az - uz * 0.1 - ux * 0.12,
            ],
            3,
          ),
        );
        const arrowMat = new THREE.MeshBasicMaterial({
          color: '#e96a55',
          side: THREE.DoubleSide,
          depthTest: false,
        });
        this.materials.add(arrowMat);
        this.rings.add(this.mesh(arrowGeo, arrowMat));
      }
    }
    const collectionCity =
      s.mode === 'collect' ? s.view?.cities.find((c) => c.position === s.city) : undefined;
    const collectionChoices = collectionCity?.choices ?? [];
    const collector = s.game?.players.find((p) => p.id === s.seat);
    const collectionSignature = JSON.stringify([
      collectionCity,
      s.selection,
      s.ballcourts,
      collector?.resources,
      collector?.resource_limit,
    ]);
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
        const triggered = displayed.map((c) =>
          collectionBonusIndicators(
            c,
            s.selection,
            collectionCity!,
            Number(!!s.ballcourts && !!collectionCity?.ballcourts),
          ),
        );
        const waste = displayed.map((c) =>
          collectionStorageWaste(
            c,
            s.selection,
            collectionCity!,
            collector?.resources,
            collector?.resource_limit,
            Number(!!s.ballcourts && !!collectionCity?.ballcourts),
          ),
        );
        const previous = this.collectionBadges.get(position);
        const label = previous?.node ?? document.createElement('button');
        if (previous) void unmount(previous.component);
        else {
          label.className = 'collect-map-label';
          label.onclick = () => {
            if (this.canPick(position)) this.pick(position);
          };
          this.bindTileHover(label, position);
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
        const bonuses = [
          ...new Set(
            [...choices.map(collectionBonusLabel), ...triggered.flat().map(collectionTriggerLabel)].filter(
              Boolean,
            ),
          ),
        ].join(', ');
        const lost = waste
          .filter((pile) => Object.values(pile).some(Boolean))
          .map((pile) =>
            Object.entries(pile)
              .map(([r, n]) => `${n} ${r}`)
              .join(' + '),
          )
          .join(' or ');
        label.title = `${position}: ${amounts}${bonuses ? ` · ${bonuses}` : ''}${lost ? ` · ${lost} will be lost to the storage limit` : ''}`;
        label.setAttribute('aria-label', `Collect at ${label.title}`);
        label.setAttribute('aria-pressed', String(selectedChoices.length > 0));
        label.classList.toggle('selected', selectedChoices.length > 0);
        label.classList.toggle(
          'storage-overflow',
          waste.some((pile) => Object.values(pile).some(Boolean)),
        );
        this.collectionBadges.set(position, {
          node: label,
          component: mount(CollectionMapBadge, {
            target: label,
            props: { piles, selected: selectedChoices.length > 0, bonuses: triggered, waste },
          }),
        });
      }
      this.collectionSignature = collectionSignature;
    }
    const moveMarkers = mapChoices.length
      ? decisionPositions
      : s.mode === 'happiness'
        ? moodPositions
        : s.mode === 'settlers'
          ? [
              ...new Set([
                ...(s.view?.units?.map((u) => u.position) ?? []),
                ...(s.view?.nomadCities ?? []),
                ...s.moveDestinations.map((d) => d.position),
                ...(s.landingTargets ?? []),
              ]),
            ]
          : abilityPositions;
    const markerSignature = JSON.stringify([
      moveMarkers,
      mapChoices,
      s.view?.decision?.name,
      ability?.key,
      s.mode,
    ]);
    if (markerSignature !== this.moveMarkerSignature) {
      this.moveMarkerSignature = markerSignature;
      for (const label of this.labelPositions.filter((l) => l.kind === 'destination')) label.node.remove();
      this.labelPositions = this.labelPositions.filter((l) => l.kind !== 'destination');
      for (const position of moveMarkers) {
        const label = document.createElement('button');
        label.className = 'map-hit-target';
        label.dataset.position = position;
        label.classList.toggle('decision-map-label', mapChoices.length > 0);
        label.textContent = '';
        const terrain = s.game.map.tiles.find(([p]) => p === position)?.[1];
        const city =
          (ability || s.mode === 'happiness') && s.view?.cities.find((city) => city.position === position);
        const description = city
          ? `${city.mood} city · Size ${city.size}`
          : terrain
            ? terrainInfo(terrain).label
            : 'Hex';
        label.dataset.description = description;
        label.setAttribute(
          'aria-label',
          `${s.mode === 'happiness' ? 'Select happiness city' : ability ? 'Choose city' : pieceDecision ? 'Choose units' : 'Choose hex'} · ${description}`,
        );
        label.onclick = () => {
          if (this.canPick(position)) this.pick(position);
        };
        this.bindTileHover(label, position);
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
    this.labelHost.classList.toggle('choosing-ability', !!ability);
    this.labelHost.classList.toggle('hide-unit-badges', !s.unitBadges);
    this.labelHost.classList.toggle('choosing-pieces', pieceDecision || s.mode === 'settlers');
    for (const label of this.labelPositions) {
      label.node.classList.toggle(
        'collection-city',
        s.mode === 'collect' &&
          (label.kind === 'city' || label.kind === 'strategy') &&
          label.position !== s.city &&
          !!s.view?.cities.some((city) => city.position === label.position),
      );
      if (label.kind !== 'collection')
        label.node.classList.toggle(
          'selected',
          mapChoices.length
            ? pieceDecision
              ? label.position === (s.decisionPosition ?? decisionPositions[0])
              : decisionSelected.includes(label.position)
            : s.mode === 'happiness'
              ? !!s.happinessSteps?.[label.position]
              : ability
                ? label.position === s.abilityCity
                : label.position === (label.kind === 'destination' ? s.moveTarget : focusedPosition),
        );
      if (label.kind === 'destination' && ability)
        label.node.setAttribute('aria-pressed', String(label.position === s.abilityCity));
      if (label.kind === 'destination' && s.mode === 'happiness')
        label.node.setAttribute('aria-pressed', String(!!s.happinessSteps?.[label.position]));
      if (label.kind === 'destination' && mapChoices.length) {
        label.node.setAttribute(
          'aria-pressed',
          String(
            pieceDecision
              ? label.position === (s.decisionPosition ?? decisionPositions[0])
              : decisionSelected.includes(label.position),
          ),
        );
      }
      label.node.disabled = !this.canPick(label.position);
    }
    if (
      s.topDown !== this.topDown ||
      s.strategyMap !== this.strategyMap ||
      guideChanged ||
      homePreferenceChanged ||
      (s.homeAtBottom && orientationChanged && !s.playback)
    ) {
      this.topDown = s.topDown;
      this.strategyMap = s.strategyMap;
      this.reset();
    }
    const interactionSignature = JSON.stringify([
      s.mode,
      s.tilePanel,
      this.interactionPositions,
      decisionPositions,
    ]);
    if (interactionSignature !== this.interactionSignature) {
      this.interactionSignature = interactionSignature;
      this.updateViewport();
      this.centerInteraction();
      if (exploration) this.locateExploration();
    }
    this.setHovered(this.hovered);
    this.invalidate();
  }
  destroy() {
    this.settleMotion?.();
    this.placement.stop();
    this.resourceOverlay.dispose();
    this.disposed = true;
    this.tileTooltip.destroy();
    this.clearCollectionBadges();
    for (const badge of this.unitBadges) void unmount(badge);
    this.unitBadges = [];
    this.seaOverlay.dispose();
    this.combatOverlay.dispose();
    this.explorationOverlay.dispose();
    this.explorationLabel.remove();
    void unmount(this.combatIcon);
    this.combatLabel.remove();
    cancelAnimationFrame(this.frame);
    this.resize.disconnect();
    this.panelObserver.disconnect();
    this.controls.dispose();
    this.civilizationFlags.dispose();
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
