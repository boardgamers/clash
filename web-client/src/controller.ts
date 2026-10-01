import { get, writable } from 'svelte/store';
import { ChatController } from '@boardgamers/protocol/chat';
import type { ViewerCommands } from '@boardgamers/protocol/viewer';
import type { Bridge, Choice, Game, Move, RecruitSelection, Session, View, MapPick, Pile } from './types';
import { journal } from './model';
import { loadBridge } from './bridge';
import { readPreferences } from './preferences';
import { GameAudio, moveSound } from './audio';
import { CardDrawTracker } from './card-draws';
import { canMoveOnMap, moveOrigins } from './map-actions';
import { movementBonus } from './movement-bonus';
import {
  researchDecision,
  mapDecisionOptions,
  mapDecisionIndex,
  toggleDecisionSelection,
} from './decision-controls';
export class Controller {
  readonly session = writable<Session>({
    decisionSelection: [],
    replacements: [],
    collectVariant: 'Collect',
    tilePanel: false,
    cityTab: 'build',
    collectionTile: null,
    moveTarget: null,
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
  private quotedActionPayment: Pile | null = null;
  private raw = '';
  private moveCache = new Map<string, Session['moveDestinations']>();
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
    this.quotedActionPayment = null;
    this.audio.play('error');
    this.patch({ error: String(error), pending: false, automaticPayment: false });
  }
  async load(raw: unknown) {
    if (typeof raw !== 'string') throw new Error('Clash expects a serialized game state.');
    if (!this.engine) {
      this.engine = await loadBridge();
    }
    if (this.destroyed) return;
    const old = get(this.session);
    // The platform can resend an unchanged snapshot after seat metadata or a reconnect.
    // It is not an acknowledgement of a pending move and must preserve local selections.
    if (raw === this.raw && old.view) return;
    const game = JSON.parse(raw) as Game;
    const changed = raw !== this.raw;
    this.raw = raw;
    this.moveCache.clear();
    const view = JSON.parse(this.engine.webView(raw, old.seat)) as View;
    const quotedPayment = this.quotedActionPayment;
    this.quotedActionPayment = null;
    let automaticPayment: Move | null = null;
    const decision = view.decision;
    const field = decision?.fields[0];
    // Continue only the exact fee already accepted on the initiating button.
    // Optional purchases, different prices and alternative payments stay visible.
    if (
      old.pending &&
      changed &&
      quotedPayment &&
      old.seat === view.activePlayer &&
      decision?.name === 'Pay for action' &&
      !decision.reward &&
      !decision.options.length &&
      decision.fields.length === 1 &&
      !field!.optional &&
      field!.choices?.length === 1
    ) {
      const payment = field!.choices[0];
      const resources = new Set([...Object.keys(payment), ...Object.keys(quotedPayment)]);
      if (
        [...resources].every((r) => (payment[r as keyof Pile] ?? 0) === (quotedPayment[r as keyof Pile] ?? 0))
      ) {
        try {
          automaticPayment = this.query<{ action: Move }>({
            kind: 'decision',
            values: [],
            payments: [payment],
          }).action;
        } catch {
          /* Keep the payment controls if the engine rejects the continuation. */
        }
      }
    }
    const researchChoice = researchDecision(view);
    const newDecision = changed || JSON.stringify(view.decision) !== JSON.stringify(old.view?.decision);
    const drawn = this.cardDraws.update(old.seat, game, view);
    const bonus = movementBonus(game);
    const bonusGranted =
      !!old.game &&
      old.seat === view.activePlayer &&
      this.submittedMove !== 'Undo' &&
      bonus &&
      bonus.key !== movementBonus(old.game)?.key;
    const city = view.cities.some((c) => c.position === old.city)
      ? old.city
      : (view.cities[0]?.position ?? null);
    clearTimeout(this.refreshTimer);
    this.patch({
      game,
      view,
      decisionSelection: newDecision ? [] : old.decisionSelection,
      decisionPosition: newDecision ? null : old.decisionPosition,
      selectedAdvance: newDecision && researchChoice ? null : old.selectedAdvance,
      tilePanel: old.pending || changed ? false : old.tilePanel,
      collectionTile: null,
      seaRouteStart: game.map.tiles.some(([p, t]) => p === old.seaRouteStart && t === 'Water')
        ? old.seaRouteStart
        : null,
      city,
      focus: old.focus ?? city,
      pending: false,
      automaticPayment: !!automaticPayment,
      error: '',
      selection: [],
      preview: null,
      recruits: {},
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      recruitPreview: null,
      replacements: [],
      collectVariant: view.collectActions?.[0]?.value ?? 'Collect',
      destination: null,
      explorationRotation: view.explorationDecision?.choices[0]?.rotation ?? null,
      explorationPreview: null,
      selectedSettler: view.settlers.some((u) => u.id === old.selectedSettler)
        ? old.selectedSettler
        : (view.settlers[0]?.id ?? null),
      mode: researchChoice
        ? newDecision || !researchDecision(old.view)
          ? 'research'
          : old.mode
        : view.stopMovement
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
    this.commands.replaceLog(journal(game, view).map((entry) => entry.text));
    if (view.decision || view.objectiveDecision || view.choiceDecision || view.explorationDecision)
      this.closeActivity();
    const recruitedMover = bonusGranted
      ? view.units?.find(
          (u) =>
            !old.view?.units?.some((previous) => previous.id === u.id) &&
            this.movementDestinations([u.id]).length,
        )
      : undefined;
    this.selectUnits(
      recruitedMover
        ? [recruitedMover.id]
        : old.selectedUnits.filter((id) => view.units?.some((u) => u.id === id)),
    );
    if (view.stopMovement && !get(this.session).moveDestinations.length) {
      const next = view.units?.find((unit) => this.movementDestinations([unit.id]).length);
      if (next) this.selectUnits([next.id]);
    }
    if (bonusGranted) {
      this.audio.play('research');
      this.notify(`${bonus.source} · ${bonus.label}`);
    } else if (drawn.length) {
      this.audio.play('draw');
    } else if (old.pending && changed && !automaticPayment) {
      this.audio.play(moveSound(this.submittedMove));
      this.notify('Game updated.');
    }
    this.submittedMove = null;
    if (automaticPayment) this.submit(automaticPayment);
  }
  setPlayer(index?: number) {
    if (get(this.session).seat === index) return;
    this.quotedActionPayment = null;
    this.moveCache.clear();
    this.cardDraws.reset();
    this.patch({
      seat: index,
      automaticPayment: false,
      cardDraws: [],
      wondersOpen: false,
      scorePlayer: null,
      decisionSelection: [],
      decisionPosition: null,
      unitPosition: null,
      tilePanel: false,
      collectionTile: null,
      moveTarget: null,
      mode: 'overview',
      selection: [],
      preview: null,
      selectedAdvance: null,
      objectivesOpen: false,
      recruits: {},
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
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
      this.patch({
        view,
        mode: researchDecision(view) ? 'research' : view.stopMovement ? 'settlers' : 'overview',
        city: view.cities[0]?.position ?? null,
        focus: view.cities[0]?.position ?? null,
      });
      if (view.stopMovement) {
        const next = view.units?.find((unit) => this.movementDestinations([unit.id]).length);
        if (next) this.selectUnits([next.id]);
      }
    }
  }
  selectCity(position: string) {
    this.patch({
      city: position,
      focus: position,
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      selection: [],
      preview: null,
      error: '',
      mode: 'overview',
      recruits: {},
      recruitPreview: null,
    });
  }
  selectTile(position: string, pick: MapPick = { kind: 'tile' }) {
    const s = get(this.session);
    if (mapDecisionOptions(s.view?.decision).length) {
      if (s.view!.decision!.options.some((o) => o.mapTarget)) {
        this.focusDecisionPosition(position);
        return;
      }
      const index = mapDecisionIndex(s.view!.decision!, position, pick);
      if (index >= 0) this.selectDecisionOption(index);
      return;
    }
    if (
      s.pending ||
      s.view?.decision ||
      s.view?.explorationDecision ||
      s.view?.choiceDecision ||
      s.view?.objectiveDecision
    )
      return;
    this.audio.play('select');
    if (s.seaRoutes && s.mode === 'overview') {
      if (s.game?.map.tiles.some(([p, terrain]) => p === position && terrain === 'Water'))
        this.patch({ seaRouteStart: position });
      return;
    }
    if (s.mode === 'collect') {
      this.selectCollectionTile(position);
      return;
    }
    // A destination takes priority over pieces on it (boarding or attacking).
    if (s.mode === 'settlers' && s.moveDestinations.some((d) => d.position === position)) {
      this.chooseMoveDestination(position);
      return;
    }
    if (
      s.mode === 'settlers' &&
      pick.kind !== 'unit' &&
      s.view?.units?.some((u) => u.position === position)
    ) {
      this.focusUnitPosition(position);
      return;
    }
    const units = s.view?.units?.filter((u) => u.position === position) ?? [];
    const city = s.view?.cities.find((c) => c.position === position);
    const pickedUnit = pick.player === s.seat ? units.find((u) => u.id === pick.unit) : undefined;
    const inspectLeader =
      (pick.kind === 'tile' || pick.kind === 'units') && units.some((u) => typeof u.type === 'object');
    if (
      canMoveOnMap(s.view, s.game) &&
      units.length &&
      (pickedUnit ||
        (pick.kind === 'units' && pick.player === s.seat && !inspectLeader) ||
        (!city && pick.kind === 'tile' && !inspectLeader) ||
        s.mode === 'settlers' ||
        !!s.view?.stopMovement)
    ) {
      this.openUnits([pickedUnit?.id ?? units.find((u) => u.carrier === null)?.id ?? units[0].id]);
      return;
    }
    if (city) this.selectCity(position);
    this.closeActivity();
    this.patch({ focus: position, tilePanel: true, mode: 'overview', abilitiesOpen: false, error: '' });
  }
  selectDecisionOption(index: number) {
    const s = get(this.session);
    const decision = s.view?.decision;
    if (s.pending || !decision || s.seat !== s.view?.activePlayer) return;
    this.audio.play('select');
    this.patch({
      decisionSelection: toggleDecisionSelection(decision, s.decisionSelection, index),
      error: '',
    });
  }
  focusDecisionPosition(position: string) {
    const s = get(this.session);
    if (
      s.pending ||
      s.seat !== s.view?.activePlayer ||
      !mapDecisionOptions(s.view?.decision).some((o) => o.position === position)
    )
      return;
    this.patch({ decisionPosition: position, error: '' });
  }
  focusUnitPosition(position: string) {
    const s = get(this.session);
    if (s.pending || !s.view?.units?.some((u) => u.position === position)) return;
    const current = s.unitPosition ?? s.view.units.find((u) => s.selectedUnits.includes(u.id))?.position;
    this.patch({ unitPosition: position, movingCity: null });
    if (current !== position) this.selectUnits([]);
  }
  chooseMoveDestination(position: string) {
    const s = get(this.session);
    if (s.pending) return;
    const indices = s.moveDestinations.flatMap((d, i) => (d.position === position ? [i] : []));
    this.patch({ moveTarget: position, moveDestination: indices.length === 1 ? indices[0] : null });
  }
  movementDestinations(ids: number[], city: string | null = null) {
    const s = get(this.session);
    if ((!ids.length && !city) || !canMoveOnMap(s.view, s.game)) return [];
    const key = (city ?? '') + [...ids].sort((a, b) => a - b).join(',');
    let destinations = this.moveCache.get(key);
    if (!destinations) {
      try {
        destinations = this.query<{ destinations: Session['moveDestinations'] }>({
          kind: 'movement',
          city,
          units: ids,
        }).destinations;
      } catch {
        destinations = [];
      }
      this.moveCache.set(key, destinations);
    }
    return destinations;
  }
  moveOrigins(position: string) {
    const s = get(this.session);
    return moveOrigins(s.view, s.game, position, (ids) => this.movementDestinations(ids));
  }
  openUnits(ids: number[], target: string | null = null) {
    if (get(this.session).pending) return;
    this.closeActivity();
    this.patch({
      movingCity: null,
      mode: 'settlers',
      tilePanel: false,
      seaRoutes: false,
      abilitiesOpen: false,
      error: '',
    });
    this.selectUnits(ids, target);
  }
  selectCollectionTile(position: string) {
    const s = get(this.session);
    if (s.pending) return;
    const choices =
      s.view?.cities.find((c) => c.position === s.city)?.choices.filter((c) => c.position === position) ?? [];
    this.patch({ collectionTile: choices.length > 1 ? position : null });
    if (choices.length === 1) this.toggleChoice(choices[0]);
  }
  collectFromTile(city: string, tile: string) {
    this.beginCollect(city);
    this.selectCollectionTile(tile);
  }
  showSeaRoutes(show = true) {
    this.closeActivity();
    this.patch({
      seaRoutes: show,
      tilePanel: false,
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
  beginCollect(position?: string) {
    if (get(this.session).pending) return;
    if (position) this.selectCity(position);
    this.closeActivity();
    this.patch({
      mode: 'collect',
      tilePanel: false,
      collectionTile: null,
      seaRoutes: false,
      selection: [],
      preview: null,
      error: '',
      selectedAdvance: null,
      collectVariant: get(this.session).view?.collectActions?.[0]?.value ?? 'Collect',
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      abilitiesOpen: false,
    });
  }
  switchCollectionCity(position: string) {
    const s = get(this.session);
    if (s.pending || s.city === position || !s.view?.cities.some((c) => c.position === position)) return;
    this.patch({
      city: position,
      focus: position,
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      selection: [],
      preview: null,
      collectionTile: null,
      error: '',
    });
  }
  openCities(position?: string, cityTab: Session['cityTab'] = 'build') {
    this.closeActivity();
    this.patch({
      mode: 'city',
      cityTab,
      tilePanel: false,
      abilitiesOpen: false,
      city: position ?? get(this.session).city,
      recruits: {},
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      replacements: [],
      recruitPreview: null,
      error: '',
    });
  }
  openSettlers() {
    const s = get(this.session);
    const atFocus = s.view?.units?.find((u) => u.position === s.focus && u.carrier === null);
    this.openUnits(
      s.selectedUnits.length
        ? s.selectedUnits
        : atFocus
          ? [atFocus.id]
          : s.view?.units?.[0]
            ? [s.view.units[0].id]
            : [],
    );
  }
  query<T>(input: unknown): T {
    const s = get(this.session);
    if (!this.engine || s.seat === undefined) throw new Error('Choose a player');
    return JSON.parse(this.engine.webQuery(this.raw, s.seat, JSON.stringify(input))) as T;
  }
  openNomadCity(position: string) {
    this.openUnits([]);
    this.patch({ movingCity: position, focus: position });
    this.selectUnits([]);
  }
  selectUnits(selectedUnits: number[], target: string | null = null) {
    const s = get(this.session);
    const city =
      s.movingCity &&
      s.view?.nomadCities?.includes(s.movingCity) &&
      selectedUnits.every((id) => s.view?.units?.find((u) => u.id === id)?.position === s.movingCity)
        ? s.movingCity
        : null;
    this.patch({
      movingCity: city,
      selectedUnits,
      unitPosition:
        city ?? s.view?.units?.find((u) => selectedUnits.includes(u.id))?.position ?? s.unitPosition,
      moveDestinations: this.movementDestinations(selectedUnits, city),
      moveDestination: null,
      moveTarget: null,
    });
    if (target) this.chooseMoveDestination(target);
  }
  setBallcourts(enabled: boolean) {
    const s = get(this.session);
    this.patch({ ballcourts: enabled, selection: [], preview: null, error: '' });
    if (s.mode === 'city') this.setRecruits(s.recruits);
  }
  setDraftCard(enabled: boolean) {
    this.patch({ draftCard: enabled });
    this.setRecruits(get(this.session).recruits);
  }
  setRecruits(recruits: RecruitSelection, payment?: Pile) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || !s.city || !this.engine) return;
    this.patch({ recruits, recruitPreview: null, error: '' });
    if (!Object.values(recruits).some(Boolean) && !s.draftCard) return;
    try {
      this.patch({
        recruitPreview: this.query({
          kind: 'recruit',
          attackPirates: !!s.attackPirates,
          payment,
          draftCard: !!s.draftCard,
          city: s.city,
          units: recruits,
          replaced: s.replacements,
          ballcourts: !!s.ballcourts,
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
    const capacity = city.capacity + Number(!!s.ballcourts && !!city.ballcourts);
    const selection = selected
      ? selected.times < city.maxPerTile && s.selection.reduce((sum, c) => sum + c.times, 0) < capacity
        ? s.selection.map((c) => (c === selected ? { ...c, times: c.times + 1 } : c))
        : s.selection.filter((c) => c !== selected)
      : [...s.selection, { ...choice, times: 1 }];
    if (selection.length > capacity) {
      this.patch({
        error: `Choose up to ${capacity} ${capacity === 1 ? 'tile' : 'tiles'} total. Remove a selection first.`,
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
            ballcourts: !!s.ballcourts,
          })
        : null;
      this.patch({ selection, preview, error: '' });
    } catch (error) {
      this.patch({ error: String(error) });
    }
  }
  submit(move: Move, quotedPayment?: Pile) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || s.view?.activePlayer !== s.seat) return;
    this.patch({ pending: true, error: '' });
    this.submittedMove = move;
    this.quotedActionPayment =
      quotedPayment && Object.values(quotedPayment).some(Boolean) ? { ...quotedPayment } : null;
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
    const variant = s.view?.collectActions?.find(
      (a) => JSON.stringify(a.value) === JSON.stringify(s.collectVariant),
    );
    if (s.preview) this.submit(s.preview.action, variant?.payment);
  }
  setTab(tab: 'journal' | 'chat') {
    this.chat.setOpen(tab === 'chat');
    this.patch({ tab, activityOpen: true, mode: 'overview', tilePanel: false, abilitiesOpen: false });
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
