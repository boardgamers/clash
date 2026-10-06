import { mount, unmount, tick } from 'svelte';
import { writable } from 'svelte/store';
import type { Session, Terrain } from './types';
import { tileUnitStacks } from './tile-units';
import TileHover from './TileHover.svelte';

export interface TileHoverData {
  position: string;
  terrain: Terrain;
  city?: { owner: number; civilization: string; mood: string; buildings: string[]; wonders: string[] };
  stacks: ReturnType<typeof tileUnitStacks>;
  colorBlind: boolean;
  playerColors?: string[];
  playerSymbols?: string[];
}

let nextId = 0;
export class TileTooltip {
  readonly node = document.createElement('div');
  private data = writable<TileHoverData | null>(null);
  private component: ReturnType<typeof mount>;
  private session: Session | null = null;
  private position: string | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private x = 0;
  private y = 0;
  private disposed = false;

  constructor(private host: HTMLElement) {
    this.node.id = `tile-tooltip-${nextId++}`;
    this.node.className = 'tile-hover';
    this.node.setAttribute('role', 'tooltip');
    this.node.hidden = true;
    host.append(this.node);
    this.component = mount(TileHover, { target: this.node, props: { data: this.data } });
    host.addEventListener('pointermove', this.track, true);
    host.addEventListener('pointerdown', this.hide, true);
    document.addEventListener('keydown', this.key);
  }

  private key = (event: KeyboardEvent) => {
    if (event.key === 'Escape') this.hide();
  };
  private track = (event: PointerEvent) => {
    if (event.pointerType === 'touch') return this.hide();
    this.anchor(event.clientX, event.clientY);
  };
  anchor(x: number, y: number) {
    const rect = this.host.getBoundingClientRect();
    this.x = x - rect.left;
    this.y = y - rect.top;
    if (!this.node.hidden) this.place();
  }
  update(session: Session) {
    this.session = session;
    if (this.position) this.refresh();
  }
  hover(position: string | null) {
    if (position === this.position) return;
    this.hide();
    if (!position) return;
    this.position = position;
    if (!this.refresh()) return;
    this.timer = setTimeout(async () => {
      await tick();
      if (this.disposed || this.position !== position) return;
      this.node.hidden = false;
      this.place();
    }, 250);
  }
  private refresh() {
    const session = this.session;
    const terrain = session?.game?.map.tiles.find(([p]) => p === this.position)?.[1];
    if (!session?.game || !terrain || !this.position) {
      this.hide();
      return false;
    }
    this.data.set({
      position: this.position,
      terrain,
      city:
        terrain === 'Unexplored'
          ? undefined
          : session.game.players.flatMap((player) => {
              const city = player.cities?.find((city) => city.position === this.position);
              return city
                ? [{ owner: player.id, civilization: player.civilization, mood: city.mood_state,
                    buildings: Object.entries(city.city_pieces ?? {}).filter(([key, owner]) => key !== 'wonders' && typeof owner === 'number').map(([key]) => key[0].toUpperCase() + key.slice(1)),
                    wonders: (city.city_pieces?.wonders ?? []).map((id) => session.view?.wonderCatalog?.find((w) => w.id === id)?.name ?? id),
                  }]
                : [];
            })[0],
      stacks: terrain === 'Unexplored' ? [] : tileUnitStacks(session.game.players, this.position),
      colorBlind: session.colorBlind,
      playerColors: session.playerColors,
      playerSymbols: session.playerSymbols,
    });
    void tick().then(() => {
      if (!this.disposed && !this.node.hidden) this.place();
    });
    return true;
  }
  private place() {
    const margin = 8,
      gap = 16;
    const width = this.node.offsetWidth,
      height = this.node.offsetHeight;
    const availableWidth = this.host.clientWidth,
      availableHeight = this.host.clientHeight;
    const x = this.x + gap + width <= availableWidth - margin ? this.x + gap : this.x - gap - width;
    const y = this.y + gap + height <= availableHeight - margin ? this.y + gap : this.y - gap - height;
    this.node.style.left = `${Math.max(margin, Math.min(x, availableWidth - width - margin))}px`;
    this.node.style.top = `${Math.max(margin, Math.min(y, availableHeight - height - margin))}px`;
  }
  hide = () => {
    clearTimeout(this.timer);
    this.position = null;
    this.node.hidden = true;
  };
  destroy() {
    this.disposed = true;
    this.hide();
    this.host.removeEventListener('pointermove', this.track, true);
    this.host.removeEventListener('pointerdown', this.hide, true);
    document.removeEventListener('keydown', this.key);
    void unmount(this.component);
    this.node.remove();
  }
}
