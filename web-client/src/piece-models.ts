import * as THREE from 'three';
import type { City, Player } from './types';
import { pieceStyle, type PieceStyle } from './piece-styles.ts';

export type BuildingKind = Exclude<keyof NonNullable<City['city_pieces']>, 'wonders'>;
type UnitKind = NonNullable<Player['units']>[number]['unit_type'];

// Chunky silhouettes survive the board's normal viewing distance. The board owns
// these resources so replacing a game state also disposes every mesh/material.
export class PieceModels {
  private palette = new Map<string, THREE.Material>();
  private material: (color: string) => THREE.Material;
  private mesh: (geometry: THREE.BufferGeometry, material: THREE.Material) => THREE.Mesh;
  constructor(
    material: (color: string) => THREE.Material,
    mesh: (geometry: THREE.BufferGeometry, material: THREE.Material) => THREE.Mesh,
  ) {
    this.material = material;
    this.mesh = mesh;
  }
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

  private arch(g: THREE.Group, color: string, x: number, y: number, z: number, width = 0.22) {
    this.add(g, new THREE.TorusGeometry(width / 2, 0.035, 4, 8, Math.PI), color, x, y, z);
    for (const dx of [-width / 2, width / 2]) this.box(g, color, 0.065, 0.2, 0.07, x + dx, y - 0.1, z);
  }
  private styledRoof(g: THREE.Group, style: PieceStyle, w: number, h: number, d: number, y: number) {
    const color = style.roofColor;
    if (style.roof === 'swept' || style.roof === 'pagoda') {
      const shape = new THREE.Shape();
      [
        [-w / 2, 0.07],
        [-w * 0.3, 0.04],
        [0, h],
        [w * 0.3, 0.04],
        [w / 2, 0.07],
        [w * 0.32, -0.02],
        [0, h * 0.75],
        [-w * 0.32, -0.02],
      ].forEach(([x, z], i) => (i ? shape.lineTo(x, z) : shape.moveTo(x, z)));
      shape.closePath();
      const geometry = new THREE.ExtrudeGeometry(shape, { depth: d, bevelEnabled: false });
      geometry.translate(0, 0, -d / 2);
      this.add(g, geometry, color, 0, y);
      this.box(g, style.trim, w * 0.9, 0.04, d * 0.95, 0, y - 0.04);
      if (style.roof === 'pagoda') {
        this.box(g, style.wall, w * 0.32, 0.12, d * 0.6, 0, y + h * 0.65);
        this.roof(g, color, w * 0.62, h * 0.65, d * 0.82, y + h * 0.9);
      }
    } else if (style.roof === 'flat') {
      this.box(g, color, w, 0.09, d, 0, y);
      for (const x of [-w / 2 + 0.025, w / 2 - 0.025]) this.box(g, style.wall, 0.05, 0.11, d, x, y + 0.08);
      this.box(g, style.trim, w, 0.045, 0.04, 0, y, d / 2);
    } else if (style.roof === 'stepped') {
      for (let i = 0; i < 3; i++)
        this.box(
          g,
          i === 2 ? style.trim : color,
          w * (1 - i * 0.22),
          h / 3,
          d * (1 - i * 0.22),
          0,
          y + ((i + 0.5) * h) / 3,
        );
    } else if (style.roof === 'dome') {
      const dome = this.add(
        g,
        new THREE.SphereGeometry(w / 2, 10, 5, 0, Math.PI * 2, 0, Math.PI / 2),
        color,
        0,
        y,
      );
      dome.scale.set(1, h / (w / 2), d / w);
      this.cylinder(g, style.trim, 0.035, 0.11, 0, y + h + 0.025);
    } else if (style.roof === 'thatch' || style.roof === 'tent') {
      const roof = this.add(g, new THREE.ConeGeometry(w * 0.65, h * 1.5, 10), color, 0, y + h * 0.65);
      roof.scale.z = d / w;
      if (style.roof === 'tent')
        for (const x of [-w * 0.46, w * 0.46])
          this.beam(g, style.wood, 0.018, [x, y, d * 0.3], [0, y + h * 1.4, 0]);
    } else {
      this.roof(g, color, w, h * (style.roof === 'timber' ? 1.5 : 1), d, y);
      if (style.roof === 'timber') {
        for (const z of [-d / 2, d / 2])
          for (const sign of [-1, 1])
            this.beam(g, style.trim, 0.022, [(sign * w) / 2, y, z], [-sign * w * 0.08, y + h * 1.65, z]);
      } else this.box(g, style.trim, w, 0.035, d + 0.02, 0, y - 0.025);
    }
  }
  private wallDetails(g: THREE.Group, style: PieceStyle, w: number, d: number, y: number) {
    if (style.roof === 'timber') {
      for (const x of [-w * 0.38, w * 0.38]) this.box(g, style.wood, 0.035, 0.27, 0.025, x, y, d / 2 + 0.01);
      this.beam(g, style.wood, 0.018, [-w * 0.4, y - 0.11, d / 2], [w * 0.4, y + 0.11, d / 2]);
    } else if (style.roof === 'roman') {
      for (const x of [-w * 0.26, w * 0.26]) this.arch(g, style.trim, x, y + 0.03, d / 2 + 0.02, w * 0.23);
    } else if (style.roof === 'swept' || style.roof === 'pagoda') {
      for (const x of [-w * 0.4, w * 0.4]) this.box(g, style.trim, 0.045, 0.3, 0.045, x, y, d / 2);
    } else if (style.roof === 'stepped' || style.roof === 'flat') {
      this.box(g, style.trim, w * 0.9, 0.04, 0.03, 0, y + 0.1, d / 2 + 0.02);
    }
  }

  settlement(color: string, civilization?: string) {
    const g = new THREE.Group(),
      style = pieceStyle(civilization),
      stone = style.wall;
    g.name = `${civilization ?? 'Neutral'} settlement`;
    this.box(g, stone, 0.8, 0.1, 0.68);
    if (style.roof === 'thatch' || style.roof === 'tent') {
      const wall = this.cylinder(g, stone, 0.32, 0.29, 0, 0.25, 0, 10);
      wall.scale.z = 0.8;
    } else this.box(g, stone, 0.58, 0.31, 0.44, 0, 0.25);
    this.box(
      g,
      '#4d5148',
      0.13,
      0.22,
      0.025,
      0,
      0.2,
      style.roof === 'tent' || style.roof === 'thatch' ? 0.26 : 0.23,
    );
    this.styledRoof(g, style, 0.75, 0.23, 0.62, 0.43);
    this.wallDetails(g, style, 0.58, 0.44, 0.25);
    this.box(g, stone, 0.48, 0.06, 0.18, 0, 0.045, 0.39);
    this.box(g, color, 0.58, 0.065, 0.035, 0, 0.39, 0.24);
    return g;
  }

  wonder(kind: string, owner: string) {
    const g = new THREE.Group(),
      stone = '#e2d4ad',
      trim = '#f3e8c8',
      dark = '#58645c',
      gold = '#bc9146';
    g.name = kind;
    // Wonders retain their own landmark silhouette regardless of the owning civilization.
    this.box(g, stone, 1.02, 0.06, 0.82);
    this.box(g, owner, 1.04, 0.04, 0.84, 0, 0.08);
    const pyramid = (width: number, height: number, x: number, y: number, z: number, color = stone) => {
      const part = this.add(
        g,
        new THREE.ConeGeometry(width / Math.sqrt(2), height, 4),
        color,
        x,
        y + height / 2,
        z,
      );
      part.rotation.y = Math.PI / 4;
      return part;
    };
    switch (kind) {
      case 'Pyramids':
        pyramid(0.67, 0.65, -0.14, 0.1, -0.06, '#d2b677');
        pyramid(0.4, 0.39, 0.29, 0.1, 0.19, '#e4cc90');
        pyramid(0.24, 0.23, -0.34, 0.1, 0.27, '#ead8a2');
        break;
      case 'GreatGardens':
        for (let level = 0; level < 3; level++) {
          const w = 0.9 - level * 0.23,
            d = 0.67 - level * 0.17,
            y = 0.1 + level * 0.22;
          this.box(g, stone, w, 0.2, d, 0, y + 0.1);
          this.box(g, '#66824c', w + 0.06, 0.06, d + 0.05, 0, y + 0.22);
          for (const x of [-w * 0.29, w * 0.29]) this.arch(g, trim, x, y + 0.11, d / 2 + 0.01, 0.12);
          for (const x of [-w / 2, w / 2]) this.box(g, '#3f7046', 0.075, 0.18, d * 0.6, x, y + 0.17);
        }
        this.box(g, '#65a9b5', 0.065, 0.54, 0.025, 0.08, 0.38, 0.36);
        for (const x of [-0.13, 0.14]) {
          this.cylinder(g, '#826546', 0.025, 0.18, x, 0.84);
          const leaves = this.add(g, new THREE.ConeGeometry(0.14, 0.12, 6), '#366843', x, 0.93);
          leaves.rotation.z = x;
        }
        break;
      case 'Colosseum': {
        // An open arena, with two tiers of arches instead of a solid cylinder.
        for (const y of [0.12, 0.35, 0.58]) {
          const ring = this.add(g, new THREE.TorusGeometry(0.43, 0.045, 4, 16), trim, 0, y);
          ring.rotation.x = Math.PI / 2;
          ring.scale.y = 0.7;
        }
        for (let i = 0; i < 14; i++) {
          const a = (i * Math.PI * 2) / 14;
          this.box(g, stone, 0.065, 0.43, 0.065, Math.sin(a) * 0.43, 0.34, Math.cos(a) * 0.3);
          for (const y of [0.28, 0.51]) {
            const arc = this.add(
              g,
              new THREE.TorusGeometry(0.065, 0.024, 4, 6, Math.PI),
              stone,
              Math.sin(a + Math.PI / 14) * 0.43,
              y,
              Math.cos(a + Math.PI / 14) * 0.3,
            );
            arc.rotation.y = a + Math.PI / 14;
          }
        }
        const arena = this.cylinder(g, '#bb9b65', 0.36, 0.025, 0, 0.11, 0, 16);
        arena.scale.z = 0.7;
        break;
      }
      case 'GreatLibrary':
        this.box(g, stone, 0.82, 0.35, 0.51, 0, 0.3, -0.08);
        for (const x of [-0.38, 0.38]) {
          this.box(g, trim, 0.17, 0.44, 0.62, x, 0.34);
          this.box(g, '#536f7e', 0.24, 0.07, 0.68, x, 0.58);
          this.box(g, dark, 0.075, 0.18, 0.025, x, 0.27, 0.32);
        }
        for (const x of [-0.23, -0.08, 0.08, 0.23]) this.cylinder(g, trim, 0.035, 0.32, x, 0.28, 0.3);
        this.roof(g, '#466878', 0.73, 0.21, 0.61, 0.47);
        this.box(g, trim, 0.75, 0.05, 0.66, 0, 0.46);
        this.box(g, gold, 0.14, 0.06, 0.025, 0, 0.56, 0.32);
        break;
      case 'GreatLighthouse':
        this.box(g, stone, 0.55, 0.44, 0.47, 0, 0.32);
        this.box(g, trim, 0.61, 0.07, 0.53, 0, 0.56);
        this.cylinder(g, stone, 0.2, 0.3, 0, 0.74, 0, 8);
        this.cylinder(g, trim, 0.25, 0.065, 0, 0.91, 0, 8);
        for (const x of [-0.12, 0.12])
          for (const z of [-0.12, 0.12]) this.cylinder(g, stone, 0.022, 0.18, x, 1.02, z);
        this.add(g, new THREE.OctahedronGeometry(0.085), '#f3b745', 0, 1.02);
        this.add(g, new THREE.ConeGeometry(0.23, 0.15, 8), '#b87c52', 0, 1.18);
        for (const y of [0.25, 0.43, 0.73])
          this.box(g, dark, 0.055, 0.085, 0.025, 0, y, y > 0.6 ? 0.195 : 0.24);
        break;
      case 'GreatMausoleum':
        this.box(g, trim, 0.85, 0.08, 0.68, 0, 0.14);
        this.box(g, stone, 0.68, 0.24, 0.54, 0, 0.29);
        this.box(g, trim, 0.77, 0.06, 0.62, 0, 0.43);
        this.box(g, dark, 0.4, 0.26, 0.32, 0, 0.58);
        for (const x of [-0.28, -0.09, 0.09, 0.28])
          for (const z of [-0.23, 0.23]) this.cylinder(g, trim, 0.03, 0.28, x, 0.58, z);
        this.box(g, trim, 0.75, 0.07, 0.61, 0, 0.74);
        for (let i = 0; i < 4; i++)
          this.box(g, '#8ea298', 0.67 - i * 0.13, 0.065, 0.53 - i * 0.1, 0, 0.81 + i * 0.065);
        this.add(g, new THREE.OctahedronGeometry(0.09), gold, 0, 1.1);
        break;
      case 'GreatStatue': {
        this.box(g, trim, 0.54, 0.09, 0.5, 0, 0.145);
        this.box(g, stone, 0.36, 0.27, 0.33, 0, 0.32);
        this.box(g, trim, 0.48, 0.07, 0.43, 0, 0.49);
        const bronze = '#709080';
        for (const x of [-0.085, 0.085]) this.box(g, bronze, 0.09, 0.3, 0.12, x, 0.67);
        this.add(g, new THREE.CylinderGeometry(0.13, 0.17, 0.27, 6), bronze, 0, 0.89);
        this.add(g, new THREE.IcosahedronGeometry(0.1, 0), bronze, 0, 1.09);
        this.beam(g, bronze, 0.045, [-0.12, 0.97, 0], [-0.3, 1.12, 0]);
        this.beam(g, bronze, 0.045, [0.12, 0.97, 0], [0.26, 0.77, 0.03]);
        this.cylinder(g, gold, 0.025, 0.19, -0.3, 1.16);
        this.add(g, new THREE.OctahedronGeometry(0.07), '#eac36b', -0.3, 1.28);
        break;
      }
      case 'GreatWall':
        for (let i = 0; i < 3; i++) {
          const wall = new THREE.Group();
          this.box(wall, stone, 0.39, 0.3, 0.15, 0, 0.25);
          this.box(wall, dark, 0.39, 0.035, 0.17, 0, 0.415);
          for (const x of [-0.15, 0, 0.15]) this.box(wall, trim, 0.075, 0.09, 0.055, x, 0.46, 0.07);
          wall.position.set((i - 1) * 0.32, 0, i === 1 ? 0.05 : -0.08);
          wall.rotation.y = (i - 1) * 0.4;
          g.add(wall);
        }
        for (const x of [-0.37, 0.37]) {
          this.box(g, stone, 0.22, 0.44, 0.26, x, 0.33, -0.08);
          const tower = new THREE.Group();
          this.roof(tower, '#8c7560', 0.34, 0.16, 0.37, 0.57);
          tower.position.set(x, 0, -0.08);
          g.add(tower);
          this.box(g, dark, 0.065, 0.1, 0.025, x, 0.46, 0.055);
        }
        break;
    }
    return g;
  }

  building(kind: BuildingKind, owner: string, civilization?: string) {
    const g = new THREE.Group(),
      style = pieceStyle(civilization),
      stone = style.wall,
      dark = '#48534e';
    g.name = `${civilization ?? 'Neutral'} ${kind}`;
    this.box(g, style.wood, 0.88, 0.08, 0.72);
    // Architecture belongs to the city; this foundation identifies the building's cultural owner.
    this.box(g, owner, 0.9, 0.055, 0.74, 0, 0.085);
    switch (kind) {
      case 'academy':
        // The open courtyard and paired wings identify an academy in every style.
        this.box(g, stone, 0.72, 0.25, 0.22, 0, 0.23, -0.22);
        for (const x of [-0.3, 0.3]) {
          this.box(g, stone, 0.21, 0.3, 0.58, x, 0.26);
          const wing = new THREE.Group();
          this.styledRoof(wing, style, 0.28, 0.12, 0.65, 0.43);
          wing.position.x = x;
          g.add(wing);
          this.box(g, dark, 0.085, 0.16, 0.025, x, 0.22, 0.3);
        }
        this.box(g, style.trim, 0.25, 0.025, 0.3, 0, 0.12, 0.05);
        if (civilization === 'Greece' || civilization === 'Persia')
          for (const x of [-0.12, 0.12]) this.cylinder(g, stone, 0.027, 0.22, x, 0.25, -0.08);
        if (style.roof === 'roman') this.arch(g, style.trim, 0, 0.36, -0.09, 0.23);
        break;
      case 'market':
        // Striped cloth and produce remain common to every market.
        for (const x of [-0.34, 0.34])
          for (const z of [-0.23, 0.23]) this.cylinder(g, style.wood, 0.035, 0.4, x, 0.3, z);
        for (let i = 0; i < 5; i++) {
          const canopy = new THREE.Group();
          this.roof(
            canopy,
            i % 2 ? style.cloth : style.trim,
            0.17,
            style.roof === 'tent' ? 0.23 : 0.12,
            0.66,
            0.49,
          ).rotation.y = Math.PI / 2;
          canopy.position.z = (i - 2) * 0.16;
          g.add(canopy);
        }
        if (style.roof === 'swept' || style.roof === 'pagoda')
          for (const x of [-0.37, 0.37]) this.beam(g, style.wood, 0.022, [x, 0.48, -0.38], [x, 0.62, 0.38]);
        this.box(g, style.wood, 0.65, 0.17, 0.22, 0, 0.19, 0.16);
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
        this.box(g, stone, 0.78, 0.09, 0.59, 0, 0.15);
        if (style.roof === 'stepped') {
          for (let i = 0; i < 4; i++)
            this.box(
              g,
              i % 2 ? stone : style.roofColor,
              0.75 - i * 0.14,
              0.12,
              0.58 - i * 0.1,
              0,
              0.24 + i * 0.12,
            );
          // Wide front stairs and a summit shrine distinguish temples from other civic buildings.
          for (let i = 0; i < 5; i++)
            this.box(g, stone, 0.17, 0.09, 0.12, 0, 0.18 + i * 0.095, 0.36 - i * 0.047);
          this.box(g, style.trim, 0.26, 0.22, 0.21, 0, 0.77);
          if (civilization === 'Maya') this.box(g, stone, 0.28, 0.19, 0.065, 0, 0.93, -0.035);
          if (civilization === 'Aztecs')
            for (const x of [-0.12, 0.12])
              this.box(g, x < 0 ? style.trim : style.roofColor, 0.18, 0.18, 0.24, x, 0.86);
        } else if (civilization === 'Egypt') {
          for (const x of [-0.24, 0.24]) {
            const pylon = this.add(g, new THREE.CylinderGeometry(0.17, 0.22, 0.54, 4), stone, x, 0.46, 0.06);
            pylon.rotation.y = Math.PI / 4;
            pylon.scale.z = 0.75;
            this.box(g, style.trim, 0.24, 0.05, 0.24, x, 0.71, 0.06);
          }
          this.box(g, stone, 0.36, 0.08, 0.22, 0, 0.52);
        } else if (style.roof === 'dome') {
          this.cylinder(g, stone, 0.27, 0.25, 0, 0.34, 0, 10);
          for (let i = 0; i < 4; i++)
            this.add(
              g,
              new THREE.CylinderGeometry(0.1 + (3 - i) * 0.035, 0.14 + (3 - i) * 0.04, 0.12, 8),
              i % 2 ? stone : style.roofColor,
              0,
              0.52 + i * 0.105,
            );
          this.cylinder(g, style.trim, 0.035, 0.17, 0, 0.98);
        } else {
          this.box(g, dark, 0.5, 0.34, 0.18, 0, 0.34, -0.15);
          for (const x of [-0.28, -0.095, 0.095, 0.28])
            this.cylinder(
              g,
              style.roof === 'timber' || style.roof === 'swept' || style.roof === 'pagoda'
                ? style.trim
                : stone,
              0.045,
              0.39,
              x,
              0.39,
              0.21,
            );
          this.box(g, stone, 0.81, 0.07, 0.62, 0, 0.61);
          this.styledRoof(g, style, 0.87, 0.24, 0.69, 0.65);
          if (style.roof === 'roman')
            for (const x of [-0.19, 0.19]) this.arch(g, style.trim, x, 0.51, 0.21, 0.24);
          this.box(g, stone, 0.61, 0.06, 0.18, 0, 0.08, 0.37);
        }
        break;
      case 'fortress':
        this.box(g, stone, 0.69, 0.41, 0.52, 0, 0.32);
        this.box(g, dark, 0.2, 0.24, 0.025, 0, 0.25, 0.27);
        for (const x of [-0.3, 0.3])
          for (const z of [-0.24, 0.24]) {
            if (style.roof === 'thatch' || style.roof === 'tent' || style.roof === 'timber') {
              for (const dx of [-0.06, 0.06]) {
                this.cylinder(g, style.wood, 0.055, 0.57, x + dx, 0.4, z);
                this.add(g, new THREE.ConeGeometry(0.055, 0.12, 6), style.trim, x + dx, 0.74, z);
              }
            } else {
              this.box(g, stone, 0.23, 0.59, 0.23, x, 0.4, z);
              if (style.roof === 'swept' || style.roof === 'pagoda') {
                const turret = new THREE.Group();
                this.styledRoof(turret, style, 0.31, 0.11, 0.31, 0.73);
                turret.position.set(x, 0, z);
                g.add(turret);
              } else
                for (const dx of [-0.07, 0.07])
                  for (const dz of [-0.07, 0.07])
                    this.box(g, style.trim, 0.09, 0.12, 0.09, x + dx, 0.75, z + dz);
            }
          }
        this.box(g, owner, 0.16, 0.2, 0.025, 0, 0.45, 0.275);
        break;
      case 'observatory':
        this.cylinder(g, stone, 0.3, 0.42, 0, 0.32, 0, 12);
        this.cylinder(g, style.trim, 0.34, 0.07, 0, 0.55, 0, 12);
        // All observatories retain a dome and raised brass sighting tube.
        this.add(
          g,
          new THREE.SphereGeometry(0.34, 12, 6, 0, Math.PI * 2, 0, Math.PI / 2),
          style.roofColor,
          0,
          0.59,
        );
        this.beam(g, '#d0ab61', 0.065, [0, 0.76, 0], [0.33, 1.02, 0.15]);
        this.box(g, dark, 0.12, 0.2, 0.025, 0, 0.23, 0.305);
        this.wallDetails(g, style, 0.53, 0.58, 0.34);
        if (style.roof === 'stepped')
          for (let i = 0; i < 2; i++)
            this.box(g, stone, 0.76 - i * 0.1, 0.055, 0.66 - i * 0.1, 0, 0.12 + i * 0.055);
        break;
      case 'obelisk':
        this.box(g, stone, 0.48, 0.14, 0.48, 0, 0.18);
        this.add(g, new THREE.CylinderGeometry(0.135, 0.2, 0.84, 4), stone, 0, 0.65).rotation.y = Math.PI / 4;
        this.add(g, new THREE.ConeGeometry(0.135, 0.22, 4), style.trim, 0, 1.18).rotation.y = Math.PI / 4;
        this.box(g, owner, 0.065, 0.32, 0.015, 0, 0.6, 0.115);
        for (const y of [0.4, 0.85]) this.box(g, style.trim, 0.2, 0.035, 0.19, 0, y);
        if (civilization === 'Celts' || civilization === 'Vikings')
          this.add(g, new THREE.TorusGeometry(0.17, 0.03, 4, 10), style.trim, 0, 0.94, 0.015);
        break;
      case 'port':
        // Wharf, warehouse and loading crane remain legible independently of ship style.
        for (let i = 0; i < 7; i++) this.box(g, style.wood, 0.115, 0.075, 0.82, (i - 3) * 0.12, 0.13);
        this.box(g, stone, 0.37, 0.3, 0.42, -0.19, 0.31, -0.1);
        const warehouse = new THREE.Group();
        this.styledRoof(warehouse, style, 0.46, 0.15, 0.51, 0.48);
        warehouse.position.set(-0.19, 0, -0.1);
        g.add(warehouse);
        this.box(g, style.wood, 0.065, 0.75, 0.065, 0.25, 0.52, -0.19);
        this.beam(g, style.wood, 0.045, [0.25, 0.83, -0.19], [0.25, 0.83, 0.4]);
        this.beam(g, style.wood, 0.03, [0.25, 0.5, -0.19], [0.25, 0.83, 0.3]);
        this.beam(g, dark, 0.018, [0.25, 0.83, 0.38], [0.25, 0.47, 0.38]);
        this.box(g, style.trim, 0.16, 0.16, 0.16, 0.25, 0.39, 0.38);
        break;
    }
    return g;
  }

  private shield(g: THREE.Group, style: PieceStyle, team: string, x: number, y: number, z: number) {
    if (style.shield === 'rectangular' || style.shield === 'wicker') {
      this.box(g, style.metal, 0.255, 0.3, 0.065, x, y, z);
      this.box(g, team, 0.21, 0.25, 0.075, x, y, z + 0.005);
      if (style.shield === 'wicker')
        for (const dy of [-0.075, 0, 0.075]) this.box(g, style.wood, 0.21, 0.018, 0.01, x, y + dy, z + 0.045);
      else this.box(g, style.trim, 0.025, 0.23, 0.01, x, y, z + 0.045);
    } else {
      const rim = this.cylinder(g, style.metal, 0.145, 0.055, x, y, z, 12);
      rim.rotation.x = Math.PI / 2;
      const face = this.cylinder(g, team, 0.123, 0.065, x, y, z + 0.01, 12);
      face.rotation.x = Math.PI / 2;
      if (style.shield === 'oval') {
        rim.scale.x = 0.77;
        face.scale.x = 0.77;
        rim.scale.z = 1.2;
        face.scale.z = 1.2;
      }
    }
    this.add(g, new THREE.SphereGeometry(0.043, 8, 4), style.metal, x, y, z + 0.058).scale.z = 0.4;
  }
  private helmet(g: THREE.Group, style: PieceStyle, team: string, y: number, z = 0) {
    const soft = style.helmet === 'cloth' || style.helmet === 'turban';
    const helmet = this.add(
      g,
      new THREE.SphereGeometry(0.105, 8, 4, 0, Math.PI * 2, 0, Math.PI / 2),
      soft ? style.cloth : style.metal,
      0,
      y,
      z,
    );
    helmet.scale.y = style.helmet === 'cap' ? 1.4 : 1;
    switch (style.helmet) {
      case 'horned':
        this.box(g, style.metal, 0.027, 0.14, 0.027, 0, y + 0.01, z + 0.09);
        for (const side of [-1, 1]) {
          const curve = new THREE.CatmullRomCurve3([
            new THREE.Vector3(side * 0.085, y + 0.035, z),
            new THREE.Vector3(side * 0.17, y + 0.045, z),
            new THREE.Vector3(side * 0.23, y + 0.11, z),
            new THREE.Vector3(side * 0.245, y + 0.225, z),
          ]);
          const horn = new THREE.TubeGeometry(curve, 8, 0.04, 6, false);
          const vertices = horn.getAttribute('position');
          for (let i = 0; i < vertices.count; i++) {
            const progress = Math.floor(i / 7) / 8;
            const center = curve.getPointAt(progress);
            const taper = 1 - progress * 0.97;
            vertices.setXYZ(
              i,
              center.x + (vertices.getX(i) - center.x) * taper,
              center.y + (vertices.getY(i) - center.y) * taper,
              center.z + (vertices.getZ(i) - center.z) * taper,
            );
          }
          horn.computeVertexNormals();
          this.add(g, horn, '#f4e4bf');
        }
        break;
      case 'crest':
      case 'legion':
        this.box(
          g,
          team,
          style.helmet === 'legion' ? 0.19 : 0.045,
          0.11,
          style.helmet === 'legion' ? 0.045 : 0.21,
          0,
          y + 0.115,
          z,
        );
        for (const x of [-0.073, 0.073]) this.box(g, style.metal, 0.035, 0.09, 0.06, x, y - 0.025, z + 0.035);
        break;
      case 'lamellar':
        this.add(g, new THREE.ConeGeometry(0.065, 0.12, 8), style.metal, 0, y + 0.11, z);
        this.add(g, new THREE.SphereGeometry(0.037, 6, 4), team, 0, y + 0.19, z);
        break;
      case 'kabuto':
        this.box(g, style.metal, 0.25, 0.035, 0.19, 0, y - 0.015, z - 0.045);
        for (const sign of [-1, 1])
          this.beam(g, '#d7b66c', 0.019, [0, y + 0.05, z + 0.1], [sign * 0.12, y + 0.17, z + 0.1]);
        break;
      case 'cloth':
        for (const x of [-0.085, 0.085]) this.box(g, style.cloth, 0.065, 0.17, 0.09, x, y - 0.03, z - 0.025);
        this.box(g, style.trim, 0.18, 0.035, 0.1, 0, y + 0.055, z);
        break;
      case 'tiara':
        this.add(g, new THREE.CylinderGeometry(0.05, 0.09, 0.16, 8), style.cloth, 0, y + 0.09, z);
        this.cylinder(g, style.trim, 0.093, 0.035, 0, y + 0.04, z);
        break;
      case 'turban':
        this.add(g, new THREE.TorusGeometry(0.087, 0.03, 5, 10), style.cloth, 0, y + 0.035, z).rotation.x =
          Math.PI / 2;
        this.add(g, new THREE.SphereGeometry(0.028, 6, 4), team, 0, y + 0.06, z + 0.095);
        break;
      case 'nasal':
        this.box(g, style.metal, 0.025, 0.16, 0.025, 0, y + 0.01, z + 0.09);
        break;
      case 'feather':
        for (let i = -2; i <= 2; i++) {
          const plume = this.add(
            g,
            new THREE.ConeGeometry(0.032, 0.2 - Math.abs(i) * 0.02, 4),
            i % 2 ? style.trim : team,
            i * 0.045,
            y + 0.14,
            z - 0.05,
          );
          plume.rotation.z = -i * 0.2;
        }
        break;
      case 'cap':
        this.cylinder(g, style.cloth, 0.109, 0.04, 0, y, z);
        break;
    }
  }
  private ship(g: THREE.Group, style: PieceStyle, owner: string, pirate: boolean, civilization?: string) {
    const kind = pirate ? 'merchant' : style.ship;
    const length = kind === 'longship' ? 0.58 : kind === 'canoe' ? 0.56 : 0.47;
    const width = kind === 'junk' ? 0.24 : kind === 'canoe' ? 0.14 : 0.19;
    const outline = new THREE.Shape();
    outline.moveTo(0, -length);
    outline.lineTo(width, -length * 0.55);
    outline.lineTo(width, length * 0.6);
    outline.lineTo(0, length);
    outline.lineTo(-width, length * 0.6);
    outline.lineTo(-width, -length * 0.55);
    outline.closePath();
    this.add(
      g,
      new THREE.ExtrudeGeometry(outline, { depth: 0.14, bevelEnabled: false }),
      style.wood,
      0,
      0.14,
    ).rotation.x = Math.PI / 2;
    this.box(g, style.cloth, width * 1.5, 0.025, length * 1.35, 0, 0.145);
    for (const x of [-width, width]) this.box(g, owner, 0.025, 0.06, length * 1.1, x, 0.18);
    if (kind === 'canoe') {
      for (const z of [-0.25, 0, 0.25]) this.box(g, style.wood, width * 1.8, 0.035, 0.08, 0, 0.18, z);
      for (const side of [-1, 1]) {
        this.beam(g, style.wood, 0.022, [0, 0.2, 0], [side * 0.32, 0.08, 0.22]);
        const paddle = this.box(g, style.trim, 0.075, 0.025, 0.16, side * 0.3, 0.09, 0.2);
        paddle.rotation.y = -side * 0.6;
      }
      this.cylinder(g, style.wood, 0.018, 0.36, 0, 0.3, -0.28);
      this.box(g, owner, 0.17, 0.17, 0.018, 0.07, 0.41, -0.28);
      return;
    }
    if (kind === 'longship' || kind === 'reed') {
      for (const sign of [-1, 1]) {
        this.beam(g, style.wood, 0.032, [0, 0.13, sign * length * 0.8], [0, 0.34, sign * length]);
        if (kind === 'longship') this.box(g, style.trim, 0.07, 0.09, 0.12, 0, 0.36, sign * length);
        else this.beam(g, style.trim, 0.025, [0, 0.32, sign * length], [0, 0.41, sign * length * 0.9]);
      }
    }
    if (kind === 'galley' || kind === 'longship') {
      for (const side of [-1, 1])
        for (const z of [-0.25, -0.08, 0.09, 0.26])
          this.beam(
            g,
            style.wood,
            0.014,
            [side * width * 0.7, 0.17, z],
            [side * (width + 0.15), 0.035, z + 0.1],
          );
      if (kind === 'galley')
        this.beam(g, style.metal, 0.035, [0, 0.08, length * 0.8], [0, 0.065, length + 0.14]);
      if (kind === 'longship')
        for (const side of [-1, 1])
          for (const z of [-0.2, 0, 0.2]) {
            const shield = this.cylinder(
              g,
              z === 0 ? owner : style.trim,
              0.07,
              0.025,
              side * (width + 0.01),
              0.21,
              z,
              8,
            );
            shield.rotation.z = Math.PI / 2;
          }
    }
    this.cylinder(g, style.wood, 0.022, 0.68, 0, 0.48);
    if (kind === 'junk' || kind === 'dhow') {
      const sail = new THREE.Shape();
      if (kind === 'dhow') {
        sail.moveTo(-0.24, 0.22);
        sail.lineTo(0.22, 0.78);
        sail.lineTo(0.22, 0.25);
      } else {
        sail.moveTo(-0.22, 0.3);
        sail.lineTo(-0.19, 0.76);
        sail.lineTo(0.14, 0.7);
        sail.lineTo(0.23, 0.38);
      }
      sail.closePath();
      this.add(g, new THREE.ExtrudeGeometry(sail, { depth: 0.018, bevelEnabled: false }), style.cloth);
      if (kind === 'junk') {
        for (const y of [0.35, 0.45, 0.55, 0.65]) this.box(g, style.wood, 0.36, 0.019, 0.025, 0, y, 0.016);
        this.box(g, owner, 0.06, 0.38, 0.024, -0.04, 0.51, 0.023);
        this.box(g, style.wall, 0.3, 0.14, 0.25, 0, 0.25, -0.23);
        if (civilization === 'Japan') {
          const cabin = new THREE.Group();
          this.roof(cabin, style.roofColor, 0.35, 0.1, 0.3, 0.34);
          cabin.position.z = -0.23;
          g.add(cabin);
        }
      } else this.beam(g, style.wood, 0.018, [-0.24, 0.22, 0.025], [0.22, 0.78, 0.025]);
    } else {
      const sailColor = pirate ? '#394546' : style.cloth;
      this.box(g, sailColor, 0.4, 0.4, 0.018, 0, 0.55);
      if (kind === 'longship')
        for (const x of [-0.16, 0, 0.16]) this.box(g, owner, 0.07, 0.4, 0.025, x, 0.55);
      else if (pirate) {
        const skull = new THREE.Shape();
        skull.moveTo(-0.065, -0.015);
        skull.bezierCurveTo(-0.14, 0.055, -0.1, 0.15, 0, 0.15);
        skull.bezierCurveTo(0.1, 0.15, 0.14, 0.055, 0.065, -0.015);
        skull.lineTo(0.055, -0.065);
        skull.lineTo(-0.055, -0.065);
        skull.closePath();
        for (const x of [-0.045, 0.045]) {
          const eye = new THREE.Path();
          eye.absellipse(x, 0.06, 0.027, 0.033, 0, Math.PI * 2, true);
          skull.holes.push(eye);
        }
        const nose = new THREE.Path();
        nose.moveTo(0, 0.035);
        nose.lineTo(-0.016, 0.002);
        nose.lineTo(0.016, 0.002);
        nose.closePath();
        skull.holes.push(nose);
        for (const side of [-1, 1]) {
          const mark = this.add(g, new THREE.ShapeGeometry(skull), '#f3e6c7', 0, 0.51, side * 0.015);
          mark.rotation.y = side < 0 ? Math.PI : 0;
        }
      } else this.box(g, owner, 0.1, 0.31, 0.025, 0, 0.55);
      this.beam(g, style.wood, 0.025, [-0.24, 0.76, 0], [0.24, 0.76, 0]);
      if (civilization === 'Carthage')
        for (const x of [-0.1, 0.1]) this.box(g, style.trim, 0.025, 0.32, 0.025, x, 0.55);
      if (civilization === 'Rome') this.box(g, owner, 0.32, 0.06, 0.027, 0, 0.55);
    }
  }

  unit(kind: UnitKind, owner: string, pirate = false, civilization?: string) {
    pirate ||= civilization === 'Pirates';
    const g = new THREE.Group(),
      style = pieceStyle(civilization),
      skin = '#e9c7a0',
      metal = style.metal,
      dark = '#354d53',
      gold = '#dfb553';
    const team = pirate ? '#394546' : owner;
    const head = (x: number, y: number, z: number) =>
      this.add(g, new THREE.SphereGeometry(0.075, 8, 6), skin, x, y, z);
    g.name = `${civilization ?? 'Neutral'} ${typeof kind === 'string' ? kind : kind.Leader}`;
    if (kind === 'Ship') {
      this.ship(g, style, team, pirate, civilization);
      return g;
    }
    const mounted = kind === 'Cavalry' || kind === 'Elephant';
    const base = this.cylinder(g, dark, mounted ? 0.26 : 0.2, 0.045);
    base.scale.z = mounted ? 1.32 : 1;
    const inset = this.cylinder(g, team, mounted ? 0.235 : 0.177, 0.03, 0, 0.055);
    inset.scale.z = mounted ? 1.32 : 1;
    if (kind === 'Settler') {
      // Robe, civilian headwear and a pack keep settlers distinct from soldiers.
      this.add(g, new THREE.CylinderGeometry(0.075, 0.13, 0.27, 7), style.cloth, 0, 0.21);
      this.box(g, team, 0.16, 0.16, 0.15, 0, 0.3);
      this.box(g, style.wood, 0.19, 0.2, 0.13, 0, 0.28, -0.12);
      head(0, 0.43, 0);
      if (
        civilization === 'China' ||
        civilization === 'Japan' ||
        civilization === 'Greece' ||
        civilization === 'Rome'
      ) {
        this.cylinder(g, style.cloth, 0.15, 0.035, 0, 0.48);
        this.add(
          g,
          new THREE.ConeGeometry(0.105, civilization === 'China' ? 0.14 : 0.075, 8),
          style.cloth,
          0,
          0.54,
        );
      } else if (civilization === 'Vikings' || civilization === 'Celts' || civilization === 'Huns') {
        this.add(g, new THREE.ConeGeometry(0.11, 0.18, 8), style.cloth, 0, 0.54);
        this.box(g, style.cloth, 0.22, 0.18, 0.04, 0, 0.35, -0.12);
      } else {
        const civilian = {
          ...style,
          helmet: civilization === 'India' ? ('turban' as const) : ('cloth' as const),
        };
        this.helmet(g, civilian, team, 0.46);
      }
      this.beam(g, '#82633f', 0.023, [0.14, 0.07, 0.08], [0.14, 0.36, 0.1]);
    } else if (kind === 'Infantry') {
      for (const x of [-0.057, 0.057]) this.box(g, dark, 0.075, 0.14, 0.1, x, 0.14);
      this.box(g, metal, 0.23, 0.22, 0.16, 0, 0.32);
      head(0, 0.48, 0);
      this.helmet(g, style, team, 0.49);
      this.shield(g, style, team, -0.075, 0.29, 0.145);
      if (style.helmet === 'lamellar' || style.helmet === 'kabuto')
        for (const y of [0.25, 0.31, 0.37]) this.box(g, style.trim, 0.23, 0.025, 0.025, 0, y, -0.09);
      if (civilization === 'Aztecs' || civilization === 'Maya')
        this.box(g, style.cloth, 0.24, 0.12, 0.17, 0, 0.22);
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
      this.helmet(g, style, team, rider + 0.19, -0.04);
      if (elephant) {
        if (civilization === 'India' || civilization === 'Persia' || civilization === 'Carthage') {
          for (const x of [-0.16, 0.16]) this.box(g, style.trim, 0.035, 0.15, 0.23, x, rider - 0.08, -0.04);
          this.box(g, style.trim, 0.35, 0.13, 0.035, 0, rider - 0.08, -0.17);
        }
        this.box(g, style.trim, 0.13, 0.07, 0.045, 0, 0.59, 0.43);
      } else if (civilization === 'Huns' || civilization === 'Persia') {
        this.add(
          g,
          new THREE.TorusGeometry(0.13, 0.018, 4, 8, Math.PI),
          style.wood,
          0.15,
          rider + 0.03,
          0.04,
        ).rotation.z = -Math.PI / 2;
      } else {
        this.beam(g, style.wood, 0.015, [0.13, rider - 0.08, 0.04], [0.13, rider + 0.3, 0.16]);
        this.box(g, team, 0.12, 0.09, 0.02, 0.18, rider + 0.23, 0.15);
      }
    } else {
      // Leaders retain their full cape, gold trim and broad standard with faction headwear.
      this.add(g, new THREE.CylinderGeometry(0.11, 0.18, 0.35, 6), team, 0, 0.25);
      this.box(g, gold, 0.16, 0.19, 0.13, 0, 0.35, 0.07);
      head(0, 0.51, 0.035);
      this.helmet(g, style, team, 0.54, 0.035);
      this.cylinder(g, gold, 0.112, 0.035, 0, 0.57, 0.035);
      this.cylinder(g, '#796341', 0.022, 0.76, 0.2, 0.43, -0.06);
      this.box(g, gold, 0.24, 0.29, 0.03, 0.3, 0.69, -0.06);
      this.box(g, team, 0.19, 0.23, 0.04, 0.3, 0.7, -0.06);
    }
    return g;
  }
}
