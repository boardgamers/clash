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
    replacements: [],
    collectVariant: 'Collect',
    selectedUnits: [],
    moveDestinations: [],
    moveDestination: null,
    cardsOpen: false,
    abilitiesOpen: false,
    seaRoutes: false,
    seaRouteStart: null,
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
    unitBadges: false,
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
  toggleUnitBadges() {
    const unitBadges = !get(this.session).unitBadges;
    if (this.commands.updatePreference('unitBadges', unitBadges)) this.patch({ unitBadges });
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
      seaRouteStart: game.map.tiles.some(([p, t]) => p === old.seaRouteStart && t === 'Water')
        ? old.seaRouteStart
        : null,
      city,
      focus: old.focus ?? city,
      pending: false,
      error: '',
      selection: [],
      preview: null,
      recruits: {},
      recruitPreview: null,
      replacements: [],
      collectVariant: view.collectActions?.[0]?.value ?? 'Collect',
      destination: null,
      explorationRotation: view.explorationDecision?.choices[0]?.rotation ?? null,
      explorationPreview: null,
      selectedSettler: view.settlers.some((u) => u.id === old.selectedSettler)
        ? old.selectedSettler
        : (view.settlers[0]?.id ?? null),
      mode: view.stopMovement
        ? 'settlers'
        : old.pending ||
            view.decision ||
            view.objectiveDecision ||
            view.choiceDecision ||
            view.explorationDecision
          ? 'overview'
          : old.mode,
      objectivesOpen:
        view.decision || view.objectiveDecision || view.choiceDecision || view.explorationDecision
          ? false
          : old.objectivesOpen,
      cardsOpen: old.pending || view.decision ? false : old.cardsOpen,
      wondersOpen: old.pending || view.decision ? false : old.wondersOpen,
      abilitiesOpen:
        old.pending || view.decision || (!view.specialActions?.length && !view.influence?.length)
          ? false
          : old.abilitiesOpen,
      cardDraws: [...old.cardDraws, ...drawn].filter((draw) =>
        draw.kind === 'wonder'
          ? view.wonderCards.some((card) => card.id === draw.card.id)
          : view.objectiveCards.some((card) => card.id === draw.card.id),
      ),
    });
    this.commands.replaceLog(journal(game).map((entry) => entry.text));
    if (view.decision || view.objectiveDecision || view.choiceDecision || view.explorationDecision)
      this.closeActivity();
    this.selectUnits(old.selectedUnits.filter((id) => view.units?.some((u) => u.id === id)));
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
      selectedUnits: [],
      moveDestinations: [],
      moveDestination: null,
      cardsOpen: false,
      abilitiesOpen: false,
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
    if (s.seaRoutes && s.mode === 'overview') {
      if (s.game?.map.tiles.some(([p, terrain]) => p === position && terrain === 'Water'))
        this.patch({ seaRouteStart: position });
      return;
    }
    const city = s.view?.cities.find((c) => c.position === position);
    if (s.mode === 'settlers') {
      const index = s.moveDestinations.findIndex((d) => d.position === position);
      if (index >= 0) this.patch({ moveDestination: index });
      else {
        const unit = s.view?.units?.find((u) => u.position === position);
        if (unit) this.selectUnits([unit.id]);
      }
    } else if (s.mode === 'collect') {
      const choice = s.view?.cities
        .find((c) => c.position === s.city)
        ?.choices.find((c) => c.position === position);
      if (choice) this.toggleChoice(choice);
    } else if (city) this.selectCity(position);
    else this.patch({ focus: position });
  }
  showSeaRoutes(show = true) {
    this.closeActivity();
    this.patch({
      seaRoutes: show,
      seaRouteStart: null,
      mode: 'overview',
      error: '',
      selection: [],
      preview: null,
    });
  }
  nextSeaRoute(direction: number) {
    const s = get(this.session);
    const water = (s.game?.map.tiles ?? [])
      .filter(([, terrain]) => terrain === 'Water')
      .map(([p]) => p)
      .sort();
    if (!water.length) return;
    const index = water.indexOf(s.seaRouteStart ?? '');
    const next =
      index < 0 ? (direction > 0 ? 0 : water.length - 1) : (index + direction + water.length) % water.length;
    this.patch({ seaRouteStart: water[next] });
  }
  beginCollect() {
    this.closeActivity();
    this.patch({
      mode: 'collect',
      selection: [],
      preview: null,
      error: '',
      selectedAdvance: null,
      collectVariant: get(this.session).view?.collectActions?.[0]?.value ?? 'Collect',
      abilitiesOpen: false,
    });
  }
  openCities(position?: string) {
    this.closeActivity();
    this.patch({
      mode: 'city',
      abilitiesOpen: false,
      city: position ?? get(this.session).city,
      recruits: {},
      replacements: [],
      recruitPreview: null,
      error: '',
    });
  }
  openSettlers() {
    this.closeActivity();
    const s = get(this.session);
    this.patch({
      mode: 'settlers',
      abilitiesOpen: false,
      selectedSettler: s.selectedSettler ?? s.view?.settlers[0]?.id ?? null,
      destination: null,
      error: '',
    });
    this.selectUnits(
      s.selectedUnits.length ? s.selectedUnits : s.view?.units?.[0] ? [s.view.units[0].id] : [],
    );
  }
  query<T>(input: unknown): T {
    const s = get(this.session);
    if (!this.engine || s.seat === undefined) throw new Error('Choose a player');
    return JSON.parse(this.engine.webQuery(this.raw, s.seat, JSON.stringify(input))) as T;
  }
  selectUnits(selectedUnits: number[]) {
    this.patch({ selectedUnits, moveDestinations: [], moveDestination: null });
    const s = get(this.session);
    if (
      !selectedUnits.length ||
      (!s.view?.canPlay && !s.view?.stopMovement) ||
      s.view?.decision ||
      s.view?.explorationDecision
    )
      return;
    try {
      const result = this.query<{ destinations: Session['moveDestinations'] }>({
        kind: 'movement',
        units: selectedUnits,
      });
      this.patch({ moveDestinations: result.destinations });
    } catch {
      /* Browsing units remains possible while a move is unavailable. */
    }
  }
  setRecruits(recruits: RecruitSelection) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || !s.city || !this.engine) return;
    this.patch({ recruits, recruitPreview: null, error: '' });
    if (!Object.values(recruits).some(Boolean)) return;
    try {
      this.patch({
        recruitPreview: this.query({
          kind: 'recruit',
          city: s.city,
          units: recruits,
          replaced: s.replacements,
        }),
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
      ? selected.times < city.maxPerTile && s.selection.reduce((sum, c) => sum + c.times, 0) < city.capacity
        ? s.selection.map((c) => (c === selected ? { ...c, times: c.times + 1 } : c))
        : s.selection.filter((c) => c !== selected)
      : [...s.selection, { ...choice, times: 1 }];
    if (selection.length > city.capacity) {
      this.patch({
        error: `Choose up to ${city.capacity} ${city.capacity === 1 ? 'tile' : 'tiles'} total. Remove a selection first.`,
      });
      return;
    }
    try {
      const preview = selection.length
        ? this.query<NonNullable<Session['preview']>>({
            kind: 'collect',
            city: s.city,
            selections: selection,
            variant: s.collectVariant,
          })
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
    this.patch({ tab, activityOpen: true, mode: 'overview', abilitiesOpen: false });
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
