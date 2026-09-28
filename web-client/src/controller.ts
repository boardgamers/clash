import { get, writable } from 'svelte/store';
import { ChatController } from '@boardgamers/protocol/chat';
import type { ViewerCommands } from '@boardgamers/protocol/viewer';
import type { Bridge, Choice, Game, Move, RecruitSelection, Session, View } from './types';
import { journal } from './model';
import { loadBridge } from './bridge';
import { readPreferences } from './preferences';
import { GameAudio, moveSound } from './audio';
import { CardDrawTracker } from './card-draws';
export class Controller {
  readonly session = writable<Session>({
    game: null,
    view: null,
    city: null,
    focus: null,
    mode: 'overview',
    selectedSettler: null,
    destination: null,
    explorationRotation: null,
    explorationPreview: null,
    recruits: {},
    recruitPreview: null,
    selection: [],
    preview: null,
    error: '',
    pending: false,
    tab: 'journal',
    activityOpen: false,
    help: false,
    objectivesOpen: false,
    wondersOpen: false,
    scorePlayer: null,
    cardDraws: [],
    dark: false,
    unread: 0,
    avatars: [],
    reducedMotion: false,
    colorBlind: false,
    sound: false,
    locale: 'en',
    selectedAdvance: null,
    toast: '',
    topDown: false,
  });
  readonly chat = new ChatController();
  readonly audio = new GameAudio();
  private cardDraws = new CardDrawTracker();
  private submittedMove: Move | null = null;
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
  setPreferences(preferences: Record<string, unknown>) {
    const next = readPreferences(preferences);
    this.audio.setEnabled(next.sound);
    this.patch(next);
  }
  toggleMapView() {
    const topDown = !get(this.session).topDown;
    if (this.commands.updatePreference('mapView', topDown ? '2d' : '3d')) this.patch({ topDown });
  }
  setGlobalPreference(name: 'sound' | 'colorBlind', enabled: boolean) {
    if (!this.commands.updatePreference(name, enabled)) return;
    if (name === 'sound') this.audio.setEnabled(enabled);
    this.patch({ [name]: enabled });
  }
  handleError(error: unknown) {
    this.submittedMove = null;
    this.audio.play('error');
    this.patch({ error: String(error), pending: false });
  }
  async load(raw: unknown) {
    if (typeof raw !== 'string') throw new Error('Clash expects a serialized game state.');
    if (!this.engine) {
      this.engine = await loadBridge();
    }
    if (this.destroyed) return;
    const old = get(this.session);
    const game = JSON.parse(raw) as Game;
    const changed = raw !== this.raw;
    this.raw = raw;
    const view = JSON.parse(this.engine.webView(raw, old.seat)) as View;
    const drawn = this.cardDraws.update(old.seat, game, view);
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
      recruits: {},
      recruitPreview: null,
      destination: null,
      explorationRotation: view.explorationDecision?.choices[0]?.rotation ?? null,
      explorationPreview: null,
      selectedSettler: view.settlers.some((u) => u.id === old.selectedSettler)
        ? old.selectedSettler
        : (view.settlers[0]?.id ?? null),
      mode: view.stopMovement
        ? 'settlers'
        : old.pending || view.objectiveDecision || view.choiceDecision || view.explorationDecision
          ? 'overview'
          : old.mode,
      objectivesOpen:
        view.objectiveDecision || view.choiceDecision || view.explorationDecision
          ? false
          : old.objectivesOpen,
      cardDraws: [...old.cardDraws, ...drawn].filter((draw) =>
        draw.kind === 'wonder'
          ? view.wonderCards.some((card) => card.id === draw.card.id)
          : view.objectiveCards.some((card) => card.id === draw.card.id),
      ),
    });
    this.commands.replaceLog(journal(game).map((entry) => entry.text));
    if (view.objectiveDecision || view.choiceDecision || view.explorationDecision) this.closeActivity();
    if (drawn.length) {
      this.audio.play('draw');
    } else if (old.pending && changed) {
      this.audio.play(moveSound(this.submittedMove));
      this.notify('Game updated.');
    }
    this.submittedMove = null;
  }
  setPlayer(index?: number) {
    if (get(this.session).seat !== index) {
      this.cardDraws.reset();
      this.patch({ cardDraws: [], wondersOpen: false, scorePlayer: null });
    }
    this.patch({
      seat: index,
      mode: 'overview',
      selection: [],
      preview: null,
      selectedAdvance: null,
      objectivesOpen: false,
      recruits: {},
      recruitPreview: null,
      selectedSettler: null,
      destination: null,
      explorationRotation: null,
      explorationPreview: null,
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
      recruits: {},
      recruitPreview: null,
    });
  }
  selectTile(position: string) {
    this.audio.play('select');
    const s = get(this.session);
    const city = s.view?.cities.find((c) => c.position === position);
    if (s.mode === 'settlers') {
      const unit = s.view?.settlers.find((u) => u.id === s.selectedSettler);
      if (unit?.destinations.some((d) => d.position === position)) this.patch({ destination: position });
      else {
        const selected = s.view?.settlers.find((u) => u.position === position);
        if (selected) this.patch({ selectedSettler: selected.id, destination: null });
      }
    } else if (s.mode === 'collect') {
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
  openCities(position?: string) {
    this.closeActivity();
    this.patch({
      mode: 'city',
      city: position ?? get(this.session).city,
      recruits: {},
      recruitPreview: null,
      error: '',
    });
  }
  openSettlers() {
    this.closeActivity();
    const s = get(this.session);
    this.patch({
      mode: 'settlers',
      selectedSettler: s.selectedSettler ?? s.view?.settlers[0]?.id ?? null,
      destination: null,
      error: '',
    });
  }
  setRecruits(recruits: RecruitSelection) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || !s.city || !this.engine) return;
    this.patch({ recruits, recruitPreview: null, error: '' });
    if (!Object.values(recruits).some(Boolean)) return;
    try {
      this.patch({
        recruitPreview: JSON.parse(
          this.engine.webRecruitPreview(this.raw, s.seat, s.city, JSON.stringify(recruits)),
        ),
      });
    } catch (error) {
      this.patch({ error: String(error) });
    }
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
    this.submittedMove = move;
    if (!this.commands.move(JSON.stringify(move))) {
      this.handleError('The action could not be sent. Please try again.');
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
    this.audio.destroy();
  }
}
