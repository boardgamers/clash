import * as THREE from 'three';
import type { City, Player } from './types';

export type BuildingKind = Exclude<keyof NonNullable<City['city_pieces']>, 'wonders'>;
type UnitKind = NonNullable<Player['units']>[number]['unit_type'];

// Chunky silhouettes survive the board's normal viewing distance. The board owns
// these resources so replacing a game state also disposes every mesh/material.
export class PieceModels {
  private palette = new Map<string, THREE.Material>();
  constructor(
    private material: (color: string) => THREE.Material,
    private mesh: (geometry: THREE.BufferGeometry, material: THREE.Material) => THREE.Mesh,
  ) {}
  private add(group: THREE.Group, geometry: THREE.BufferGeometry, color: string, x = 0, y = 0, z = 0) {
    let mat = this.palette.get(color);
    if (!mat) this.palette.set(color, (mat = this.material(color)));
    const part = this.mesh(geometry, mat);
    part.position.set(x, y, z);
    group.add(part);
    return part;
  }
  private box(g: THREE.Group, color: string, w: number, h: number, d: number, x = 0, y = h / 2, z = 0) {
    return this.add(g, new THREE.BoxGeometry(w, h, d), color, x, y, z);
  }
  private cylinder(g: THREE.Group, color: string, r: number, h: number, x = 0, y = h / 2, z = 0, sides = 8) {
    return this.add(g, new THREE.CylinderGeometry(r, r, h, sides), color, x, y, z);
  }
  private beam(g: THREE.Group, color: string, r: number, from: number[], to: number[]) {
    const a = new THREE.Vector3(...from),
      b = new THREE.Vector3(...to);
    const mesh = this.cylinder(g, color, r, a.distanceTo(b));
    mesh.position.copy(a.clone().add(b).multiplyScalar(0.5));
    mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), b.sub(a).normalize());
    return mesh;
  }
  private roof(g: THREE.Group, color: string, w: number, h: number, d: number, y: number) {
    const profile = new THREE.Shape();
    profile.moveTo(-w / 2, 0);
    profile.lineTo(0, h);
    profile.lineTo(w / 2, 0);
    profile.closePath();
    const geometry = new THREE.ExtrudeGeometry(profile, { depth: d, bevelEnabled: false });
    geometry.translate(0, 0, -d / 2);
    return this.add(g, geometry, color, 0, y);
  }

  settlement(color: string) {
    const g = new THREE.Group(),
      stone = '#ead7ad';
    this.box(g, stone, 0.8, 0.1, 0.68);
    this.box(g, stone, 0.58, 0.31, 0.44, 0, 0.25);
    this.box(g, '#4d5148', 0.13, 0.22, 0.02, 0, 0.2, 0.23);
    const roof = this.add(g, new THREE.ConeGeometry(0.54, 0.24, 4), '#a86042', 0, 0.55);
    roof.rotation.y = Math.PI / 4;
    roof.scale.z = 0.8;
    for (const x of [-0.18, 0.18]) this.box(g, '#4d5148', 0.07, 0.11, 0.025, x, 0.29, 0.23);
    this.box(g, stone, 0.48, 0.06, 0.18, 0, 0.045, 0.39);
    this.box(g, color, 0.58, 0.065, 0.035, 0, 0.39, 0.24);
    return g;
  }

  building(kind: BuildingKind, owner: string) {
    const g = new THREE.Group(),
      stone = '#ead7ad',
      shade = '#c3b18f',
      dark = '#48534e';
    g.name = kind;
    this.box(g, shade, 0.88, 0.08, 0.72);
    // The colored foundation also identifies culturally influenced buildings.
    this.box(g, owner, 0.9, 0.055, 0.74, 0, 0.085);
    switch (kind) {
      case 'academy':
        // Open courtyard between two long, tiled wings.
        this.box(g, stone, 0.72, 0.25, 0.22, 0, 0.23, -0.22);
        for (const x of [-0.3, 0.3]) {
          this.box(g, stone, 0.21, 0.3, 0.58, x, 0.26);
          const wing = new THREE.Group();
          this.roof(wing, '#7b9390', 0.28, 0.15, 0.65, 0.43);
          wing.position.x = x;
          g.add(wing);
          this.box(g, dark, 0.085, 0.16, 0.02, x, 0.22, 0.3);
        }
        this.box(g, '#aebbae', 0.25, 0.025, 0.3, 0, 0.12, 0.05);
        break;
      case 'market':
        // Broad striped awning, open sides and a counter of goods.
        for (const x of [-0.34, 0.34])
          for (const z of [-0.23, 0.23]) this.cylinder(g, '#796249', 0.035, 0.4, x, 0.3, z);
        for (let i = 0; i < 5; i++) {
          const canopy = new THREE.Group();
          this.roof(canopy, i % 2 ? '#f5e7c4' : '#b86e48', 0.17, 0.12, 0.66, 0.49).rotation.y = Math.PI / 2;
          canopy.position.z = (i - 2) * 0.16;
          g.add(canopy);
        }
        this.box(g, '#92714a', 0.65, 0.17, 0.22, 0, 0.19, 0.16);
        for (let i = 0; i < 3; i++)
          this.add(
            g,
            new THREE.DodecahedronGeometry(0.08),
            ['#dfb958', '#80964e', '#c7774e'][i],
            (i - 1) * 0.2,
            0.31,
            0.17,
          );
        break;
      case 'temple':
        // Tall pediment and exposed colonnade, distinct from the closed settlement.
        this.box(g, stone, 0.78, 0.09, 0.59, 0, 0.15);
        this.box(g, dark, 0.5, 0.34, 0.18, 0, 0.34, -0.15);
        for (const x of [-0.28, -0.095, 0.095, 0.28]) this.cylinder(g, stone, 0.045, 0.39, x, 0.39, 0.21);
        this.box(g, stone, 0.81, 0.07, 0.62, 0, 0.61);
        this.roof(g, '#b57658', 0.87, 0.26, 0.69, 0.65);
        this.box(g, stone, 0.61, 0.06, 0.18, 0, 0.08, 0.37);
        break;
      case 'fortress':
        this.box(g, '#acb6ac', 0.69, 0.41, 0.52, 0, 0.32);
        this.box(g, dark, 0.2, 0.24, 0.025, 0, 0.25, 0.27);
        for (const x of [-0.3, 0.3])
          for (const z of [-0.24, 0.24]) {
            this.box(g, '#cbd0bf', 0.23, 0.59, 0.23, x, 0.4, z);
            for (const dx of [-0.07, 0.07])
              for (const dz of [-0.07, 0.07]) this.box(g, '#cbd0bf', 0.09, 0.12, 0.09, x + dx, 0.75, z + dz);
          }
        this.box(g, owner, 0.16, 0.2, 0.025, 0, 0.45, 0.275);
        break;
      case 'observatory':
        this.cylinder(g, stone, 0.3, 0.42, 0, 0.32, 0, 12);
        this.cylinder(g, shade, 0.34, 0.07, 0, 0.55, 0, 12);
        this.add(
          g,
          new THREE.SphereGeometry(0.34, 12, 6, 0, Math.PI * 2, 0, Math.PI / 2),
          '#598981',
          0,
          0.59,
        );
        // A raised brass sighting tube keeps the dome recognizable from above.
        this.beam(g, '#d0ab61', 0.065, [0, 0.76, 0], [0.33, 1.02, 0.15]);
        this.box(g, dark, 0.12, 0.2, 0.025, 0, 0.23, 0.305);
        break;
      case 'obelisk':
        this.box(g, stone, 0.48, 0.14, 0.48, 0, 0.18);
        this.add(g, new THREE.CylinderGeometry(0.135, 0.2, 0.84, 4), stone, 0, 0.65).rotation.y = Math.PI / 4;
        this.add(g, new THREE.ConeGeometry(0.135, 0.22, 4), '#d0ab61', 0, 1.18).rotation.y = Math.PI / 4;
        this.box(g, owner, 0.065, 0.32, 0.015, 0, 0.6, 0.115);
        break;
      case 'port':
        // Wharf, warehouse and a large loading crane; no ship in the building.
        for (let i = 0; i < 7; i++) this.box(g, '#a68a62', 0.115, 0.075, 0.82, (i - 3) * 0.12, 0.13);
        this.box(g, stone, 0.37, 0.3, 0.42, -0.19, 0.31, -0.1);
        const roof = this.roof(g, '#637f83', 0.46, 0.18, 0.51, 0.48);
        roof.position.set(-0.19, 0.48, -0.1);
        this.box(g, '#776047', 0.065, 0.75, 0.065, 0.25, 0.52, -0.19);
        this.beam(g, '#776047', 0.045, [0.25, 0.83, -0.19], [0.25, 0.83, 0.4]);
        this.beam(g, '#776047', 0.03, [0.25, 0.5, -0.19], [0.25, 0.83, 0.3]);
        this.beam(g, dark, 0.018, [0.25, 0.83, 0.38], [0.25, 0.47, 0.38]);
        this.box(g, '#b69559', 0.16, 0.16, 0.16, 0.25, 0.39, 0.38);
        break;
    }
    return g;
  }

  unit(kind: UnitKind, owner: string, pirate = false) {
    const g = new THREE.Group(),
      skin = '#e9c7a0',
      metal = '#71878e',
      dark = '#354d53',
      gold = '#dfb553';
    const team = pirate ? '#394546' : owner;
    const head = (x: number, y: number, z: number) =>
      this.add(g, new THREE.SphereGeometry(0.075, 8, 6), skin, x, y, z);
    if (kind === 'Ship') {
      const outline = new THREE.Shape();
      outline.moveTo(0, -0.43);
      outline.lineTo(0.18, -0.22);
      outline.lineTo(0.18, 0.23);
      outline.lineTo(0, 0.4);
      outline.lineTo(-0.18, 0.23);
      outline.lineTo(-0.18, -0.22);
      outline.closePath();
      const hull = this.add(
        g,
        new THREE.ExtrudeGeometry(outline, { depth: 0.14, bevelEnabled: false }),
        team,
        0,
        0.13,
      );
      hull.rotation.x = Math.PI / 2;
      this.box(g, '#bb9b6b', 0.24, 0.025, 0.49, 0, 0.14);
      this.cylinder(g, '#6c573d', 0.023, 0.67, 0, 0.46);
      this.box(g, pirate ? '#394546' : '#f3e6c7', 0.38, 0.38, 0.025, 0, 0.54);
      this.box(g, pirate ? '#f3e6c7' : owner, 0.1, 0.31, 0.03, 0, 0.54);
      this.beam(g, '#806746', 0.025, [-0.24, 0.75, 0], [0.24, 0.75, 0]);
      return g;
    }
    const mounted = kind === 'Cavalry' || kind === 'Elephant';
    const base = this.cylinder(g, dark, mounted ? 0.26 : 0.2, 0.045);
    base.scale.z = mounted ? 1.32 : 1;
    const inset = this.cylinder(g, team, mounted ? 0.235 : 0.177, 0.03, 0, 0.055);
    inset.scale.z = mounted ? 1.32 : 1;
    if (kind === 'Settler') {
      // Soft robe, broad straw hat and pack: a civilian rather than a soldier.
      this.add(g, new THREE.CylinderGeometry(0.075, 0.13, 0.27, 7), '#efe1bf', 0, 0.21);
      this.box(g, team, 0.16, 0.16, 0.15, 0, 0.3);
      this.box(g, '#977345', 0.19, 0.2, 0.13, 0, 0.28, -0.12);
      head(0, 0.43, 0);
      this.cylinder(g, '#d7b678', 0.15, 0.035, 0, 0.48);
      this.add(g, new THREE.ConeGeometry(0.105, 0.095, 8), '#d7b678', 0, 0.54);
      this.beam(g, '#82633f', 0.023, [0.14, 0.07, 0.08], [0.14, 0.36, 0.1]);
    } else if (kind === 'Infantry') {
      for (const x of [-0.057, 0.057]) this.box(g, dark, 0.075, 0.14, 0.1, x, 0.14);
      this.box(g, metal, 0.23, 0.22, 0.16, 0, 0.32);
      head(0, 0.48, 0);
      this.add(g, new THREE.SphereGeometry(0.105, 8, 4, 0, Math.PI * 2, 0, Math.PI / 2), metal, 0, 0.49);
      this.box(g, team, 0.05, 0.09, 0.21, 0, 0.6);
      // A broad shield remains visible even when the spear is subpixel-sized.
      this.box(g, '#d2b56e', 0.255, 0.3, 0.065, -0.075, 0.29, 0.145);
      this.box(g, team, 0.21, 0.25, 0.075, -0.075, 0.29, 0.15);
      this.add(g, new THREE.SphereGeometry(0.043, 8, 4), '#d2b56e', -0.075, 0.29, 0.19).scale.z = 0.4;
      this.cylinder(g, '#705a3b', 0.022, 0.58, 0.17, 0.38, 0.01);
      this.add(g, new THREE.ConeGeometry(0.055, 0.15, 4), metal, 0.17, 0.745, 0.01);
    } else if (mounted) {
      const elephant = kind === 'Elephant',
        animal = elephant ? '#969f97' : '#8d6148';
      const legHeight = elephant ? 0.22 : 0.25;
      for (const x of [-0.13, 0.13])
        for (const z of [-0.18, 0.19])
          this.box(g, animal, elephant ? 0.105 : 0.065, legHeight, 0.085, x, 0.08 + legHeight / 2, z);
      this.box(g, animal, elephant ? 0.37 : 0.22, elephant ? 0.28 : 0.19, 0.48, 0, elephant ? 0.4 : 0.36);
      this.box(g, team, elephant ? 0.39 : 0.25, 0.065, 0.25, 0, elephant ? 0.55 : 0.48, -0.045);
      if (elephant) {
        this.add(g, new THREE.DodecahedronGeometry(0.19), animal, 0, 0.49, 0.29);
        for (const x of [-0.22, 0.22]) this.box(g, '#aab1a1', 0.14, 0.29, 0.08, x, 0.47, 0.24);
        this.beam(g, animal, 0.065, [0, 0.47, 0.41], [0, 0.18, 0.48]);
        this.beam(g, animal, 0.06, [0, 0.18, 0.48], [0, 0.17, 0.58]);
        for (const x of [-0.12, 0.12]) this.beam(g, '#f4e4bf', 0.028, [x, 0.37, 0.37], [x, 0.31, 0.56]);
      } else {
        this.beam(g, animal, 0.095, [0, 0.36, 0.17], [0, 0.62, 0.26]);
        this.box(g, animal, 0.14, 0.15, 0.23, 0, 0.64, 0.32);
        this.box(g, '#4d423a', 0.07, 0.27, 0.06, 0, 0.53, 0.17);
        for (const x of [-0.05, 0.05])
          this.add(g, new THREE.ConeGeometry(0.028, 0.09, 4), animal, x, 0.76, 0.24);
        this.beam(g, '#4d423a', 0.035, [0, 0.4, -0.24], [0, 0.18, -0.31]);
      }
      const rider = elephant ? 0.67 : 0.59;
      this.box(g, team, 0.14, 0.19, 0.13, 0, rider, -0.04);
      head(0, rider + 0.17, -0.04);
      this.add(
        g,
        new THREE.SphereGeometry(0.083, 8, 4, 0, Math.PI * 2, 0, Math.PI / 2),
        metal,
        0,
        rider + 0.19,
        -0.04,
      );
    } else {
      // Leaders have a full cape, gold crown and a broad standard.
      this.add(g, new THREE.CylinderGeometry(0.11, 0.18, 0.35, 6), team, 0, 0.25);
      this.box(g, gold, 0.16, 0.19, 0.13, 0, 0.35, 0.07);
      head(0, 0.51, 0.035);
      this.cylinder(g, gold, 0.1, 0.055, 0, 0.57, 0.035);
      for (const x of [-0.065, 0, 0.065])
        this.add(g, new THREE.ConeGeometry(0.035, 0.1, 4), gold, x, 0.645, 0.035);
      this.cylinder(g, '#796341', 0.022, 0.76, 0.2, 0.43, -0.06);
      this.box(g, gold, 0.24, 0.29, 0.03, 0.3, 0.69, -0.06);
      this.box(g, team, 0.19, 0.23, 0.04, 0.3, 0.7, -0.06);
    }
    return g;
  }
}
