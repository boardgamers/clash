import { get, writable } from 'svelte/store';
import { ChatController } from '@boardgamers/protocol/chat';
import type { ViewerCommands } from '@boardgamers/protocol/viewer';
import type { Bridge, Choice, Game, Move, Session, View } from './types';
import { journal } from './model';
export class Controller {
  readonly session = writable<Session>({
    game: null,
    view: null,
    city: null,
    focus: null,
    mode: 'overview',
    selection: [],
    preview: null,
    error: '',
    pending: false,
    tab: 'journal',
    activityOpen: false,
    help: false,
    objectivesOpen: false,
    dark: false,
    unread: 0,
    avatars: [],
    reducedMotion: false,
    colorBlind: false,
    locale: 'en',
    selectedAdvance: null,
    toast: '',
    topDown: false,
  });
  readonly chat = new ChatController();
  private raw = '';
  private engine: Bridge | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  private destroyed = false;
  private chatOff: () => void;
  constructor(
    readonly commands: ViewerCommands<string>,
    readonly assetBase: URL,
  ) {
    this.chat.setOpen(false);
    this.chatOff = this.chat.subscribe(() => this.patch({ unread: this.chat.unread }));
  }
  patch(patch: Partial<Session>) {
    this.session.update((s) => ({ ...s, ...patch }));
  }
  async load(raw: unknown) {
    if (typeof raw !== 'string') throw new Error('Clash expects a serialized game state.');
    if (!this.engine) {
      const engine = (await import(
        /* @vite-ignore */ new URL('engine/server.js', this.assetBase).href
      )) as Bridge;
      await engine.default();
      this.engine = engine;
    }
    if (this.destroyed) return;
    const old = get(this.session);
    const game = JSON.parse(raw) as Game;
    this.raw = raw;
    const view = JSON.parse(this.engine.webView(raw, old.seat)) as View;
    const city = view.cities.some((c) => c.position === old.city)
      ? old.city
      : (view.cities[0]?.position ?? null);
    clearTimeout(this.refreshTimer);
    this.patch({
      game,
      view,
      city,
      focus: old.focus ?? city,
      pending: false,
      error: '',
      selection: [],
      preview: null,
      mode: old.pending || view.objectiveDecision ? 'overview' : old.mode,
      objectivesOpen: view.objectiveDecision ? false : old.objectivesOpen,
    });
    this.commands.replaceLog(journal(game).map((entry) => entry.text));
    if (old.pending) this.notify('Game updated.');
  }
  setPlayer(index?: number) {
    this.patch({
      seat: index,
      mode: 'overview',
      selection: [],
      preview: null,
      selectedAdvance: null,
      objectivesOpen: false,
    });
    if (this.engine && this.raw) {
      const view = JSON.parse(this.engine.webView(this.raw, index)) as View;
      this.patch({ view, city: view.cities[0]?.position ?? null, focus: view.cities[0]?.position ?? null });
    }
  }
  selectCity(position: string) {
    this.patch({
      city: position,
      focus: position,
      selection: [],
      preview: null,
      error: '',
      mode: 'overview',
    });
  }
  selectTile(position: string) {
    const s = get(this.session);
    const city = s.view?.cities.find((c) => c.position === position);
    if (s.mode === 'collect') {
      const choice = s.view?.cities
        .find((c) => c.position === s.city)
        ?.choices.find((c) => c.position === position);
      if (choice) this.toggleChoice(choice);
    } else if (city) this.selectCity(position);
    else this.patch({ focus: position });
  }
  beginCollect() {
    this.closeActivity();
    this.patch({ mode: 'collect', selection: [], preview: null, error: '', selectedAdvance: null });
  }
  toggleChoice(choice: Choice) {
    const s = get(this.session);
    if (s.pending || !s.view?.canPlay || s.seat === undefined || !s.city || !this.engine) return;
    const selected = s.selection.find(
      (c) => c.position === choice.position && JSON.stringify(c.pile) === JSON.stringify(choice.pile),
    );
    const city = s.view.cities.find((c) => c.position === s.city)!;
    const selection = selected
      ? s.selection.filter((c) => c !== selected)
      : [...s.selection, { ...choice, times: 1 }];
    if (selection.length > city.capacity) {
      this.patch({ error: `This city can collect from ${city.capacity} tiles. Remove a selection first.` });
      return;
    }
    try {
      const preview = selection.length
        ? JSON.parse(this.engine.webCollectPreview(this.raw, s.seat, s.city, JSON.stringify(selection)))
        : null;
      this.patch({ selection, preview, error: '' });
    } catch (error) {
      this.patch({ error: String(error) });
    }
  }
  submit(move: Move) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || s.view?.activePlayer !== s.seat) return;
    this.patch({ pending: true, error: '' });
    if (!this.commands.move(JSON.stringify(move))) {
      this.patch({ pending: false, error: 'The action could not be sent. Please try again.' });
      return;
    }
    this.refreshTimer = setTimeout(() => {
      this.commands.fetchState();
      this.patch({ error: 'Waiting for the game to confirm your action…' });
    }, 8000);
  }
  collect() {
    const s = get(this.session);
    if (s.preview) this.submit(s.preview.action);
  }
  setTab(tab: 'journal' | 'chat') {
    this.chat.setOpen(tab === 'chat');
    this.patch({ tab, activityOpen: true, mode: 'overview' });
  }
  closeActivity() {
    this.chat.setOpen(false);
    this.patch({ activityOpen: false });
  }
  notify(toast: string) {
    clearTimeout(this.timer);
    this.patch({ toast });
    this.timer = setTimeout(() => this.patch({ toast: '' }), 4000);
  }
  destroy() {
    this.destroyed = true;
    clearTimeout(this.timer);
    clearTimeout(this.refreshTimer);
    this.chatOff();
  }
}
