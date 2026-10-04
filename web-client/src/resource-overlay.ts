import * as THREE from 'three';
import { mount, unmount } from 'svelte';
import ResourceGainBadge from './ResourceGainBadge.svelte';
import { positionXY } from './model';
import { playerColor, type Session } from './types';
import type { ResourceMarker } from './resource-playback';

export class ResourceOverlay {
  private nodes: { node: HTMLDivElement; at: THREE.Vector3; component: ReturnType<typeof mount> }[] = [];
  private timer: ReturnType<typeof setTimeout> | undefined;
  private host: HTMLDivElement;
  constructor(parent: HTMLElement) {
    this.host = document.createElement('div');
    Object.assign(this.host.style, {
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
      overflow: 'hidden',
      zIndex: '3',
    });
    this.host.className = 'resource-gains';
    parent.append(this.host);
  }
  show(markers: ResourceMarker[], s: Session, animate: boolean) {
    this.clear();
    const groups = new Map<string, ResourceMarker[]>();
    for (const marker of markers) {
      const key = `${marker.player}:${marker.position}`;
      groups.set(key, [...(groups.get(key) ?? []), marker]);
    }
    for (const group of groups.values()) {
      const { player, position } = group[0];
      const node = document.createElement('div');
      node.dataset.position = position;
      Object.assign(node.style, { position: 'absolute', left: '0', top: '0', visibility: 'hidden' });
      node.style.setProperty('--gain-player', playerColor(player, s.colorBlind, s.playerColors));
      this.host.append(node);
      const component = mount(ResourceGainBadge, {
        target: node,
        props: {
          markers: group,
          animate,
          transient: !s.playback,
          civilization: s.game?.players.find((p) => p.id === player)?.civilization ?? 'Player',
        },
      });
      const [x, z] = positionXY(position);
      this.nodes.push({ node, component, at: new THREE.Vector3(x, 1.3, z) });
    }
    if (!s.playback && markers.length) this.timer = setTimeout(() => this.clear(), 2300);
  }
  project(camera: THREE.Camera, width: number, height: number) {
    for (const { node, at } of this.nodes) {
      const p = at.clone().project(camera);
      node.style.transform = `translate(${((p.x + 1) * width) / 2}px,${((1 - p.y) * height) / 2}px)`;
      node.style.visibility = p.z < -1 || p.z > 1 ? 'hidden' : 'visible';
    }
  }
  clear() {
    clearTimeout(this.timer);
    for (const { node, component } of this.nodes) {
      void unmount(component);
      node.remove();
    }
    this.nodes = [];
  }
  dispose() {
    this.clear();
    this.host.remove();
  }
}
