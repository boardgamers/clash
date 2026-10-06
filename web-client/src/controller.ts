import { influencePaymentMatches } from './influence';
import { defaultCity, type CollectionPotential } from './default-city';
import { get, writable } from 'svelte/store';
import { ChatController } from '@boardgamers/protocol/chat';
import type { ViewerCommands } from '@boardgamers/protocol/viewer';
import type { Bridge, Choice, Game, Move, RecruitSelection, Session, View, MapPick, Pile } from './types';
import { journal } from './model';
import { loadBridge } from './bridge';
import { readPreferences } from './preferences';
import { GameAudio, moveSound } from './audio';
import { CardDrawTracker } from './card-draws';
import { canMoveOnMap, moveOrigins, passengerLandings } from './map-actions';
import { movementBonus } from './movement-bonus';
import { activeCityAbility, groupAbilities } from './abilities';
import { recapStart, sinceLastTurn, frameAt, frameEffects } from './playback';
import { groupPlaybackFrames, playbackSteps } from './replay-actions';
import { battleCues, battleCursor, BATTLE_DURATION, type BattleCue } from './battle-playback';
import { contextualCards, type CardContext } from './contextual-cards';
import { recruitDiscardSelection, requiredRecruitDiscards } from './recruit-discards';
import { happinessTargets } from './happiness';
import { UndoPreview } from './undo-preview';
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
    pirateSpawns: false,
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
    homeAtBottom: false,
    topDown: false,
    strategyMap: false,
    unitBadges: false,
    replayAutoplay: true,
    availableOnly: false,
    skipRazeCity: false,
  });
  readonly chat = new ChatController();
  readonly audio = new GameAudio();
  private cardDraws = new CardDrawTracker();
  private submittedMove: Move | null = null;
  private quotedFreeEducation: Pile | null = null;
  private quotedActionPayment: Pile | null = null;
  private quotedInfluenceRangePayment: Pile | null = null;
  private influenceAttemptPending = false;
  private cardContinuation:
    | (Pick<
        Session,
        | 'mode'
        | 'city'
        | 'cityTab'
        | 'selection'
        | 'collectVariant'
        | 'ballcourts'
        | 'selectedAdvance'
        | 'happinessSteps'
        | 'happinessVariant'
        | 'happinessCity'
        | 'happinessLawgiver'
      > & { id: number })
    | null = null;
  private raw = '';
  private undoPreview = new UndoPreview();
  private undoRestore: Partial<Session> | null = null;
  private playerSettings: Record<string, unknown> | null = null;
  private legacySkipRaze = false;
  private migratedSkipRaze = false;
  private keepCitiesRequest: { seat: number; raw: string } | null = null;
  private moveCache = new Map<string, Session['moveDestinations']>();
  private engine: Bridge | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  private destroyed = false;
  private playbackTimer: ReturnType<typeof setTimeout> | undefined;
  private effectTimer: ReturnType<typeof setTimeout> | undefined;
  private battleTimer: ReturnType<typeof setTimeout> | undefined;
  private chatOff: () => void;
  constructor(
    readonly commands: ViewerCommands<string> & { clearReplayInfo?: () => void },
    readonly assetBase: URL,
  ) {
    this.chat.setOpen(false);
    this.chatOff = this.chat.subscribe(() => this.patch({ unread: this.chat.unread }));
  }
  patch(patch: Partial<Session>) {
    this.session.update((s) => {
      const next = { ...s, ...patch };
      if ('scorePlayer' in patch && patch.scorePlayer !== s.scorePlayer) {
        next.scoreTab = patch.scoreTab ?? 'score';
        next.scoreObjective = patch.scoreObjective ?? null;
      }
      if (!next.abilitiesOpen || next.mode !== 'overview') {
        next.abilityChoice = null;
        next.abilityCity = null;
      }
      return next;
    });
  }
  setPreferences(preferences: Record<string, unknown>) {
    const old = get(this.session);
    this.legacySkipRaze = preferences.skipRazeCity === true;
    const next = readPreferences(preferences);
    this.audio.setEnabled(next.sound);
    this.patch(next);
    if (next.analysis) this.endPlayback();
    else if (!next.replayAutoplay && old.playback?.range === 'catch-up' && old.playback.playing) {
      const playback = get(this.session).playback!;
      clearTimeout(this.playbackTimer);
      this.patch({
        playback: { ...playback, playing: false },
      });
    }
    this.migrateRazeSetting();
  }
  setSettings(settings: Record<string, unknown> | null) {
    this.playerSettings = settings;
    this.patch({ skipRazeCity: settings?.skipRazeCity === true });
    const request = this.keepCitiesRequest;
    this.keepCitiesRequest = null;
    const s = get(this.session);
    // Only the explicit "Keep all cities" checkbox also answers a current
    // decision. Receiving saved settings must never submit a background move.
    if (
      settings?.skipRazeCity === true &&
      request &&
      request.seat === s.seat &&
      request.raw === this.raw &&
      !s.analysis &&
      !s.playback &&
      !s.pending &&
      s.view?.activePlayer === s.seat &&
      s.view.decision?.name === 'Raze city' &&
      s.view.decision.endOfAge &&
      s.view.decision.min === 0
    ) {
      try {
        const { action } = this.query<{ action: Move }>({ kind: 'decision', values: [], payments: [] });
        this.submit(action);
      } catch {
        /* The regular Skip button remains available. */
      }
    }
    this.migrateRazeSetting();
  }
  private migrateRazeSetting() {
    const s = get(this.session);
    if (
      !this.legacySkipRaze ||
      this.migratedSkipRaze ||
      !this.playerSettings ||
      this.playerSettings.skipRazeCity !== undefined ||
      s.game?.players.find((p) => p.id === s.seat)?.settings?.skipRazeCity !== undefined ||
      s.analysis ||
      s.seat === undefined ||
      !s.game ||
      !s.view
    )
      return;
    // One-time migration from the old account preference, once this player's
    // saved game settings arrive. An explicit false always wins.
    this.migratedSkipRaze = true;
    this.commands.updateSetting('skipRazeCity', true);
  }
  setSkipRazeCity(enabled: boolean) {
    const s = get(this.session);
    if (s.analysis || s.playback || s.pending || s.seat === undefined) return;
    this.keepCitiesRequest = enabled ? { seat: s.seat, raw: this.raw } : null;
    if (!this.commands.updateSetting('skipRazeCity', enabled)) {
      this.keepCitiesRequest = null;
      this.patch({ error: 'Could not save the city setting. Please try again.' });
    }
  }
  setReplayAutoplay(enabled: boolean) {
    this.patch({ replayAutoplay: enabled });
    const playback = get(this.session).playback;
    if (!enabled && playback?.range === 'catch-up') {
      clearTimeout(this.playbackTimer);
      this.patch({ playback: { ...playback, playing: false } });
    }
    this.commands.updatePreference('replayAutoplay', enabled);
  }
  setAvailableOnly(enabled: boolean) {
    this.patch({ availableOnly: enabled });
    this.commands.updatePreference('availableOnly', enabled);
  }
  toggleMapView() {
    const strategyMap = !get(this.session).strategyMap;
    this.patch({ topDown: strategyMap, strategyMap });
    this.commands.updatePreference('mapView', strategyMap ? 'strategy' : '3d');
  }
  setGlobalPreference(name: 'sound' | 'colorBlind', enabled: boolean) {
    if (!this.commands.updatePreference(name, enabled)) return;
    if (name === 'sound') this.audio.setEnabled(enabled);
    this.patch({ [name]: enabled });
  }
  handleError(error: unknown) {
    clearTimeout(this.refreshTimer);
    this.restoreUndoPreview();
    this.keepCitiesRequest = null;
    this.cardContinuation = null;
    this.submittedMove = null;
    this.quotedActionPayment = null;
    this.quotedInfluenceRangePayment = null;
    this.quotedFreeEducation = null;
    this.influenceAttemptPending = false;
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
    if (game.board_history) game.board_history = { ...game.board_history, frames: groupPlaybackFrames(game) };
    const changed = raw !== this.raw;
    this.raw = raw;
    this.moveCache.clear();
    const view = JSON.parse(this.engine.webView(raw, old.seat)) as View;
    this.undoRestore = null;
    this.undoPreview.remember(game, view, old.seat, raw.length * 2);
    if (view.influenceContext) this.influenceAttemptPending = true;
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
    if (
      old.pending &&
      changed &&
      old.seat === view.activePlayer &&
      influencePaymentMatches(view, this.quotedInfluenceRangePayment)
    ) {
      try {
        automaticPayment = this.query<{ action: Move }>({
          kind: 'decision',
          values: [],
          payments: [this.quotedInfluenceRangePayment],
        }).action;
      } catch {
        /* Keep the range controls when a quote is rejected. */
      }
      this.quotedInfluenceRangePayment = null;
    } else if (!view.influenceContext || !['payment', 'range'].includes(view.influenceContext.stage)) {
      this.quotedInfluenceRangePayment = null;
    }
    if (
      old.pending &&
      changed &&
      this.quotedFreeEducation &&
      old.seat === view.activePlayer &&
      decision?.origin?.Advance === 'FreeEducation' &&
      !decision.reward &&
      !decision.options.length &&
      decision.fields.length === 1 &&
      decision.fields[0].optional
    ) {
      try {
        automaticPayment = this.query<{ action: Move }>({
          kind: 'decision',
          values: [],
          payments: [this.quotedFreeEducation],
        }).action;
      } catch {
        /* Show the regular dialog if the agreed purchase is no longer legal. */
      }
      this.quotedFreeEducation = null;
    } else if (old.pending && changed && !automaticPayment) this.quotedFreeEducation = null;
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
    const movementEnded = !!old.view?.stopMovement && !view.stopMovement;
    const turnChanged =
      !!old.game &&
      (old.game.current_player_index !== game.current_player_index ||
        old.game.round !== game.round ||
        old.game.age !== game.age);
    const resetMovement = movementEnded || turnChanged || (!view.canPlay && !view.stopMovement);
    const ability = groupAbilities(view.specialActions).find((group) => group.key === old.abilityChoice);
    clearTimeout(this.refreshTimer);
    this.patch({
      abilityChoice: ability?.key ?? null,
      abilityCity: ability?.offers.some((offer) => offer.position === old.abilityCity)
        ? old.abilityCity
        : null,
      game,
      view,
      decisionSelection: newDecision ? [] : old.decisionSelection,
      decisionPosition: newDecision ? null : old.decisionPosition,
      selectedAdvance: newDecision && researchChoice ? null : old.selectedAdvance,
      tilePanel: old.pending || changed ? false : old.tilePanel,
      collectionTile: null,
      disembarkCarriers: undefined,
      landingTargets: [],
      unitPosition: resetMovement ? null : old.unitPosition,
      movingCity: resetMovement ? null : old.movingCity,
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
      happinessSteps: {},
      happinessCity: null,
      happinessVariant: 0,
      happinessLawgiver: null,
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
              movementEnded ||
              turnChanged ||
              (old.mode === 'settlers' && resetMovement) ||
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
    const continuation = this.cardContinuation;
    if (continuation && !view.actionCards?.some((card) => card.id === continuation.id)) {
      this.cardContinuation = null;
      if (
        old.pending &&
        view.canPlay &&
        old.seat === view.activePlayer &&
        !view.decision &&
        !view.choiceDecision &&
        !view.objectiveDecision &&
        !view.explorationDecision
      ) {
        const { id: _, ...resume } = continuation;
        this.patch({ ...resume, city, cardsOpen: false });
        if (resume.mode === 'collect') {
          const variant =
            view.collectActions?.find(
              (offer) => JSON.stringify(offer.value) === JSON.stringify(resume.collectVariant),
            )?.value ?? view.collectActions?.[0]?.value;
          if (variant) this.switchCollectVariant(variant);
        }
      }
    }
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
        : resetMovement
          ? []
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
      if (!this.influenceAttemptPending) this.notify('Game updated.');
    }
    if (this.influenceAttemptPending && changed && !automaticPayment && !game.events?.length) {
      this.notify(
        game.successful_cultural_influence ? 'Cultural influence succeeded' : 'Cultural influence failed',
      );
      this.influenceAttemptPending = false;
    }
    this.submittedMove = null;
    if (automaticPayment) this.submit(automaticPayment);
    this.afterPlaybackLoad(old, game);
    this.migrateRazeSetting();
  }
  private seenKey() {
    const s = get(this.session);
    return s.game?.board_history && s.seat !== undefined
      ? `clash:board-seen:${s.game.board_history.id}:${s.seat}`
      : null;
  }
  private markSeen(cursor?: number) {
    const s = get(this.session),
      key = this.seenKey();
    if (!key || s.analysis) return;
    try {
      localStorage.setItem(
        key,
        String(
          Math.max(
            Number(localStorage.getItem(key)) || 0,
            cursor ?? s.game?.board_history?.frames.at(-1)?.cursor ?? 0,
          ),
        ),
      );
    } catch {}
  }
  private afterPlaybackLoad(old: Session, game: Game) {
    const s = get(this.session),
      frames = game.board_history?.frames ?? [];
    if (s.analysis || !frames.length) return;
    if (s.playback) {
      if (old.game?.board_history?.id !== game.board_history?.id) this.endPlayback();
      else {
        const index = frameAt(frames, s.playback.frame?.cursor ?? 0);
        if (frames[index]?.cursor !== s.playback.frame?.cursor) this.showBattles([]);
        const previous = old.game?.board_history?.frames ?? [];
        const start = frameAt(frames, previous[s.playback.start]?.cursor ?? frames[0].cursor);
        const end =
          s.playback.range === 'all'
            ? frames.length - 1
            : frameAt(frames, previous[s.playback.end]?.cursor ?? frames.at(-1)!.cursor);
        const steps = playbackSteps(game, start, end);
        const visibleIndex = steps.filter((step) => step <= index).at(-1) ?? start;
        this.patch({
          playback: {
            ...s.playback,
            steps,
            index: visibleIndex,
            start,
            end: steps.at(-1)!,
            frame: frames[visibleIndex],
            total: frames.length,
          },
        });
      }
      return;
    }
    if (!old.game) {
      let seen = 0;
      try {
        seen = Number(localStorage.getItem(this.seenKey() ?? '')) || 0;
      } catch {}
      const start = recapStart(game, s.seat, seen);
      if (start !== null) {
        this.startPlayback(true, start);
        return;
      }
    } else if (!old.game.board_history?.id || old.game.board_history?.id === game.board_history?.id) {
      const before = battleCursor(old.game);
      const after = battleCursor(game);
      if (after < before) this.showBattles([]);
      else {
        const battles = battleCues(game, before, after, s.view);
        if (battles.length) this.showBattles([...(s.battles ?? []), ...battles]);
      }
      const effects = frameEffects(game, old.game.board_history?.frames.at(-1)?.cursor ?? 0).filter(
        (e) => e.kind === 'completed' || e.kind === 'action' || e.player !== s.seat,
      );
      if (effects.length) {
        this.patch({ publicEffects: [...(s.publicEffects ?? []), ...effects] });
        this.scheduleEffect();
      }
    } else this.showBattles([]);
    this.markSeen();
  }
  replayLastTurn() {
    const s = get(this.session);
    if (!s.game || s.pending) return;
    const turn = sinceLastTurn(s.game, s.seat);
    if (turn) this.startPlayback(false, turn.start, turn.end, 'last-turn');
  }
  startPlayback(
    automatic = false,
    index = 0,
    end?: number,
    range: 'all' | 'last-turn' | 'catch-up' = automatic ? 'catch-up' : 'all',
  ) {
    const s = get(this.session);
    if (s.analysis || !s.game) return;
    clearTimeout(this.playbackTimer);
    const frames = s.game.board_history?.frames ?? [];
    const steps = playbackSteps(s.game, index, end ?? Math.max(0, frames.length - 1));
    this.dismissEffects(true);
    this.showBattles([]);
    this.patch({
      playback: {
        steps,
        frame: frames[index] ?? null,
        index,
        total: frames.length,
        start: index,
        end: steps.at(-1) ?? index,
        range,
        automatic,
        playing: automatic && s.replayAutoplay,
        animate: false,
      },
      tilePanel: false,
      activityOpen: false,
      mode: 'overview',
      cardsOpen: false,
      objectivesOpen: false,
      wondersOpen: false,
      abilitiesOpen: false,
      help: false,
      scorePlayer: null,
      seaRoutes: false,
      pirateSpawns: false,
      cardDraws: [],
    });
    this.reportPlayback();
    if (automatic && s.replayAutoplay) this.schedulePlayback();
  }
  private reportPlayback() {
    const s = get(this.session),
      frames = s.game?.board_history?.frames ?? [],
      p = s.playback;
    if (p && p.range === 'all' && frames.length)
      this.commands.setReplayInfo({
        start: frames[0].cursor,
        current: p.frame?.cursor ?? frames[0].cursor,
        end: frames[p.end].cursor,
      });
  }
  seekPlayback(cursor: number) {
    const frames = get(this.session).game?.board_history?.frames ?? [];
    if (get(this.session).playback?.range !== 'all') this.startPlayback();
    this.showPlaybackFrame(frameAt(frames, cursor), false);
  }
  stepPlayback(direction: number) {
    const p = get(this.session).playback;
    if (p) {
      if (p.range !== 'all') this.setReplayAutoplay(false);
      const steps = p.steps ?? Array.from({ length: p.end - p.start + 1 }, (_, i) => p.start + i);
      this.showPlaybackFrame(
        steps[Math.max(0, Math.min(steps.length - 1, steps.indexOf(p.index) + direction))],
        false,
      );
    }
  }
  restartPlayback() {
    const p = get(this.session).playback;
    if (p) this.showPlaybackFrame(p.start, get(this.session).replayAutoplay);
  }
  private showPlaybackFrame(index: number, playing: boolean) {
    clearTimeout(this.playbackTimer);
    const s = get(this.session),
      frames = s.game?.board_history?.frames ?? [],
      p = s.playback;
    if (!p || !frames.length) return;
    index = Math.max(p.start, Math.min(index, p.end));
    index = p.steps?.filter((step) => step <= index).at(-1) ?? index;
    const previous = p.steps?.[Math.max(0, p.steps.indexOf(index) - 1)] ?? index - 1;
    this.dismissEffects(true);
    const battles =
      index > p.start ? battleCues(s.game!, frames[previous].cursor, frames[index].cursor, s.view) : [];
    this.showBattles(battles, previous === p.index);
    this.patch({
      playback: {
        ...p,
        frame: frames[index],
        index,
        total: frames.length,
        playing: playing && index < p.end,
        animate: index !== p.index && !s.reducedMotion,
      },
      publicEffects: (index === p.start ? [] : (frames[index].effects ?? [])).map((e, i) => ({
        ...e,
        key: `replay:${frames[index].cursor}:${i}`,
      })),
    });
    this.reportPlayback();
    this.scheduleEffect();
    if (playing && index < p.end) this.schedulePlayback();
  }
  togglePlayback() {
    const s = get(this.session),
      p = s.playback;
    if (!p) return;
    if (p.range !== 'all') this.setReplayAutoplay(!p.playing);
    if (p.playing) {
      clearTimeout(this.playbackTimer);
      this.patch({ playback: { ...p, playing: false } });
    } else if (p.index >= p.end) {
      this.showPlaybackFrame(p.start, true);
    } else {
      this.patch({ playback: { ...p, playing: true } });
      this.schedulePlayback();
    }
  }
  private schedulePlayback() {
    clearTimeout(this.playbackTimer);
    const s = get(this.session),
      p = s.playback;
    if (!p?.playing || p.index >= p.end) return;
    const duration = Math.max(
      2500,
      (p.frame?.effects?.length ?? 0) * 1800,
      (s.battles?.length ?? 0) * BATTLE_DURATION + 500,
    );
    this.playbackTimer = setTimeout(() => {
      const p = get(this.session).playback;
      if (p?.playing) this.showPlaybackFrame(p.steps?.[p.steps.indexOf(p.index) + 1] ?? p.index + 1, true);
    }, duration);
  }
  endPlayback() {
    clearTimeout(this.playbackTimer);
    const s = get(this.session),
      p = s.playback;
    if (p) {
      this.dismissEffects(true);
      this.showBattles([]);
      this.patch({ playback: null });
      this.commands.clearReplayInfo?.();
      this.markSeen(s.game?.board_history?.frames[p.end]?.cursor);
    }
  }
  private scheduleEffect() {
    if (this.effectTimer || !get(this.session).publicEffects?.length) return;
    this.effectTimer = setTimeout(() => {
      this.effectTimer = undefined;
      this.dismissEffects();
    }, 1400);
  }
  private showBattles(battles: BattleCue[], animate = true) {
    clearTimeout(this.battleTimer);
    this.patch({ battles, battleAnimate: animate && !get(this.session).reducedMotion });
    this.scheduleBattle();
  }
  private scheduleBattle() {
    if (!get(this.session).battles?.length) return;
    this.battleTimer = setTimeout(() => {
      const s = get(this.session);
      if (s.playback && s.battles?.length === 1) return; // Keep the result readable when stepping manually.
      this.patch({ battles: s.battles?.slice(1) ?? [] });
      this.scheduleBattle();
    }, BATTLE_DURATION);
  }
  dismissBattles() {
    this.showBattles([]);
  }
  dismissEffects(all = false) {
    clearTimeout(this.effectTimer);
    this.effectTimer = undefined;
    this.patch({ publicEffects: all ? [] : (get(this.session).publicEffects ?? []).slice(1) });
    if (!all) this.scheduleEffect();
  }
  setPlayer(index?: number) {
    if (get(this.session).seat === index) return;
    this.restoreUndoPreview();
    this.undoPreview.clear();
    if (get(this.session).seat !== undefined) this.playerSettings = null;
    this.migratedSkipRaze = false;
    this.keepCitiesRequest = null;
    this.patch({ skipRazeCity: this.playerSettings?.skipRazeCity === true });
    this.cardContinuation = null;
    this.endPlayback();
    this.dismissEffects(true);
    this.quotedActionPayment = null;
    this.quotedInfluenceRangePayment = null;
    this.quotedFreeEducation = null;
    this.influenceAttemptPending = false;
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
      happinessSteps: {},
      happinessCity: null,
      happinessLawgiver: null,
      happinessVariant: 0,
      selectedUnits: [],
      landingTargets: [],
      disembarkCarriers: undefined,
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
      const loaded = get(this.session);
      if (index !== undefined && loaded.game) this.afterPlaybackLoad({ ...loaded, game: null }, loaded.game);
      this.migrateRazeSetting();
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
    if (get(this.session).playback) return;
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
    const ability = activeCityAbility(s);
    if (ability) {
      if (ability.offers.some((offer) => offer.position === position))
        this.patch({ abilityCity: position, error: '' });
      return;
    }
    if (s.seaRoutes && s.mode === 'overview') {
      if (s.game?.map.tiles.some(([p, terrain]) => p === position && terrain === 'Water'))
        this.patch({ seaRouteStart: position });
      return;
    }
    if (s.mode === 'collect') {
      if (position !== s.city && s.view?.cities.some((city) => city.position === position))
        this.switchCollectionCity(position);
      else this.selectCollectionTile(position);
      return;
    }
    if (s.mode === 'happiness') {
      this.toggleHappinessCity(position);
      return;
    }
    if (s.mode === 'settlers' && s.landingTargets?.includes(position)) {
      const ships = s.selectedUnits.filter((id) => s.view?.units?.find((u) => u.id === id)?.type === 'Ship');
      const landing = passengerLandings(s.view, ships, (ids) => this.movementDestinations(ids)).find(
        (d) => d.position === position,
      );
      if (landing) {
        this.patch({ disembarkCarriers: ships });
        this.selectUnits(landing.units, position);
        return;
      }
    }
    // A destination takes priority over pieces on it (boarding or attacking).
    if (s.mode === 'settlers' && s.moveDestinations.some((d) => d.position === position)) {
      this.chooseMoveDestination(position);
      return;
    }
    if (
      s.mode === 'settlers' &&
      pick.kind !== 'unit' &&
      pick.kind !== 'units' &&
      s.view?.nomadCities?.includes(position)
    ) {
      this.openNomadCity(position);
      return;
    }
    if (s.mode === 'settlers' && s.view?.units?.some((u) => u.position === position)) {
      this.focusUnitPosition(position, pick.kind === 'unit' ? pick.unit : undefined);
      return;
    }
    this.inspectTile(position);
  }
  inspectTile(position: string) {
    const s = get(this.session);
    if (s.pending || s.playback) return;
    if (s.view?.cities.some((c) => c.position === position)) this.selectCity(position);
    this.closeActivity();
    this.patch({ focus: position, tilePanel: true, mode: 'overview', abilitiesOpen: false, error: '' });
  }
  chooseAbility(key: string) {
    const s = get(this.session);
    const group = groupAbilities(s.view?.specialActions).find((group) => group.key === key);
    if (s.pending || !s.view?.canPlay || !group?.offers.every((offer) => offer.position)) return;
    this.closeActivity();
    this.patch({
      abilitiesOpen: true,
      abilityChoice: key,
      abilityCity: null,
      mode: 'overview',
      tilePanel: false,
      seaRoutes: false,
      pirateSpawns: false,
      error: '',
    });
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
      !!s.playback ||
      s.seat !== s.view?.activePlayer ||
      !mapDecisionOptions(s.view?.decision).some((o) => o.position === position)
    )
      return;
    this.patch({ decisionPosition: position, error: '' });
  }
  defaultMovementGroup(position: string, preferredUnit?: number) {
    const units = get(this.session).view?.units?.filter((u) => u.position === position) ?? [];
    const preferred = units.find((u) => u.id === preferredUnit);
    const ships = units.filter((u) => u.type === 'Ship').map((u) => u.id);
    const land = units.filter((u) => u.type !== 'Ship').map((u) => u.id);
    // Ships carry their passengers automatically. Never put both in one move payload.
    const ids = preferred ? (preferred.type === 'Ship' ? ships : land) : ships.length ? ships : land;
    // Keep groups together when an escort or leader makes movement possible.
    if (this.movementDestinations(ids).length) return ids;
    const movable = ids.filter((id) => this.movementDestinations([id]).length);
    return movable.length ? movable : ids;
  }
  focusUnitPosition(position: string, preferredUnit?: number) {
    const s = get(this.session);
    if (s.pending || s.playback || !s.view?.units?.some((u) => u.position === position)) return;
    this.patch({ unitPosition: position, movingCity: null, disembarkCarriers: undefined });
    this.selectUnits(this.defaultMovementGroup(position, preferredUnit));
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
      disembarkCarriers: undefined,
      mode: 'settlers',
      tilePanel: false,
      seaRoutes: false,
      pirateSpawns: false,
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
      pirateSpawns: false,
      tilePanel: false,
      seaRouteStart: null,
      mode: 'overview',
      error: '',
      selection: [],
      preview: null,
    });
  }
  showPirateSpawns(show = true, guide: 'pirates' | 'barbarians' = 'pirates') {
    this.closeActivity();
    const s = get(this.session);
    this.patch({
      pirateSpawns: show,
      threatGuide: guide,
      pirateSpawnPlayer: s.seat ?? s.view?.activePlayer,
      seaRoutes: false,
      seaRouteStart: null,
      tilePanel: false,
      mode: 'overview',
      focus: null,
      city: null,
      abilitiesOpen: false,
      abilityChoice: null,
      abilityCity: null,
      cardsOpen: false,
      error: '',
      selection: [],
      preview: null,
    });
  }
  setPirateSpawnPlayer(player: number) {
    if (get(this.session).view?.pirateSpawns?.some((p) => p.player === player))
      this.patch({ pirateSpawnPlayer: player });
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
    const s = get(this.session);
    if (s.pending) return;
    const variant = s.view?.collectActions?.[0]?.value ?? 'Collect';
    const city =
      position ??
      defaultCity(
        s.view,
        'collect',
        s.city,
        s.view?.canPlay ? this.query<CollectionPotential[]>({ kind: 'collectionPotential', variant }) : [],
      );
    if (city) this.selectCity(city);
    this.closeActivity();
    this.patch({
      mode: 'collect',
      tilePanel: false,
      collectionTile: null,
      seaRoutes: false,
      pirateSpawns: false,
      selection: [],
      preview: null,
      error: '',
      selectedAdvance: null,
      collectVariant: variant,
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      abilitiesOpen: false,
    });
  }
  switchCollectVariant(variant: Move) {
    const s = get(this.session);
    if (
      s.pending ||
      s.mode !== 'collect' ||
      !s.city ||
      !s.view?.canPlay ||
      !s.view.collectActions?.some((offer) => JSON.stringify(offer.value) === JSON.stringify(variant))
    )
      return;
    // Requote the same tiles and resources; a failed quote must not leave the old
    // action available or discard the player's choices.
    this.patch({ collectVariant: variant, preview: null, error: '' });
    if (!s.selection.length) return;
    try {
      this.patch({
        preview: this.query<NonNullable<Session['preview']>>({
          kind: 'collect',
          city: s.city,
          selections: s.selection,
          variant,
          ballcourts: !!s.ballcourts,
        }),
      });
    } catch (error) {
      this.patch({ error: String(error) });
    }
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
    if (cityTab === 'happiness') {
      this.beginHappiness(position);
      return;
    }
    const s = get(this.session);
    const city = position ?? defaultCity(s.view, cityTab, s.city);
    this.closeActivity();
    this.patch({
      mode: 'city',
      cityTab,
      tilePanel: false,
      abilitiesOpen: false,
      city,
      focus: city ?? s.focus,
      recruits: {},
      ballcourts: false,
      draftCard: false,
      attackPirates: false,
      replacements: [],
      recruitPreview: null,
      error: '',
    });
  }
  beginHappiness(position?: string) {
    const s = get(this.session);
    if (s.pending || s.playback || s.seat === undefined) return;
    this.closeActivity();
    this.patch({
      mode: 'happiness',
      cityTab: 'happiness',
      tilePanel: false,
      seaRoutes: false,
      pirateSpawns: false,
      abilitiesOpen: false,
      happinessSteps: {},
      happinessCity: null,
      happinessVariant: 0,
      happinessLawgiver: null,
      error: '',
    });
    if (position) this.toggleHappinessCity(position);
  }
  toggleHappinessCity(position: string) {
    const s = get(this.session);
    if (
      s.pending ||
      s.playback ||
      s.mode !== 'happiness' ||
      !s.view?.cities.some((city) => city.position === position && city.mood !== 'Happy')
    )
      return;
    this.patch({ happinessCity: position, focus: position });
    if (s.happinessSteps?.[position]) this.setHappinessCity(position, 0);
    else {
      const target = happinessTargets(s, (input) => this.query(input), position).find((t) => t.action);
      if (target) this.setHappinessCity(position, target.steps, target.lawgiver);
    }
  }
  setHappinessCity(position: string, steps: number, lawgiver = false) {
    const s = get(this.session);
    if (s.pending || s.playback || s.mode !== 'happiness') return;
    if (
      steps &&
      !happinessTargets(s, (input) => this.query(input), position).some(
        (t) => t.steps === steps && !!t.lawgiver === lawgiver && t.action,
      )
    )
      return;
    const selected = { ...s.happinessSteps };
    if (steps) selected[position] = steps;
    else delete selected[position];
    this.patch({
      happinessSteps: selected,
      happinessCity: position,
      happinessLawgiver:
        steps && lawgiver ? position : s.happinessLawgiver === position ? null : s.happinessLawgiver,
      error: '',
    });
  }
  switchHappinessVariant(index: number) {
    const s = get(this.session);
    if (s.pending || s.playback || s.mode !== 'happiness' || !s.view?.happinessActions?.[index]) return;
    this.patch({ happinessVariant: index, error: '' });
  }
  openSettlers() {
    const s = get(this.session);
    const cities = s.view?.nomadCities ?? [];
    const positions = [...new Set(s.view?.units?.map((u) => u.position) ?? [])];
    const movable = positions.filter((p) => this.movementDestinations(this.defaultMovementGroup(p)).length);
    if ((s.focus && cities.includes(s.focus)) || (!movable.length && cities.length)) {
      this.openNomadCity(s.focus && cities.includes(s.focus) ? s.focus : cities[0]);
      return;
    }
    const position = movable.find((p) => p === s.focus) ?? movable[0] ?? positions[0];
    this.openUnits(
      s.selectedUnits.length ? s.selectedUnits : position ? this.defaultMovementGroup(position) : [],
    );
  }
  query<T>(input: unknown): T {
    const s = get(this.session);
    if (!this.engine || s.seat === undefined) throw new Error('Choose a player');
    return JSON.parse(this.engine.webQuery(this.raw, s.seat, JSON.stringify(input))) as T;
  }
  openNomadCity(position: string) {
    const s = get(this.session);
    if (s.pending || s.playback || !s.view?.nomadCities?.includes(position) || !canMoveOnMap(s.view, s.game))
      return;
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
    const ships = selectedUnits.filter((id) => s.view?.units?.find((u) => u.id === id)?.type === 'Ship');
    this.patch({
      landingTargets: passengerLandings(s.view, ships, (ids) => this.movementDestinations(ids)).map(
        (d) => d.position,
      ),
      disembarkCarriers: ships.length ? undefined : s.disembarkCarriers,
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
    if (s.pending || s.playback) return;
    this.patch({ ballcourts: enabled, preview: null, error: '' });
    if (s.mode === 'collect') this.switchCollectVariant(s.collectVariant);
    if (s.mode === 'city') this.setRecruits(s.recruits);
  }
  shogunateDraftOffers() {
    const s = get(this.session);
    if (!s.view?.canPlay || s.pending || s.playback) return [];
    return s.view.cities
      .filter((c) => c.shogunateDraft)
      .flatMap((c) => {
        try {
          const preview = this.query<NonNullable<Session['recruitPreview']>>({
            kind: 'recruit',
            city: c.position,
            units: {},
            replaced: [],
            draftCard: true,
          });
          return [{ position: c.position, payment: preview.payment }];
        } catch {
          return [];
        }
      });
  }
  beginShogunateDraft(position: string) {
    if (!this.shogunateDraftOffers().some((c) => c.position === position)) return;
    this.openCities(position, 'recruit');
    this.setDraftCard(true);
  }
  setDraftCard(enabled: boolean) {
    this.patch({ draftCard: enabled });
    this.setRecruits(get(this.session).recruits);
  }
  setRecruits(recruits: RecruitSelection, payment?: Pile) {
    const s = get(this.session);
    if (s.pending || s.seat === undefined || !s.city || !this.engine) return;
    const replacements = recruitDiscardSelection(s.view, s.city, recruits, s.replacements);
    this.patch({ recruits, replacements, recruitPreview: null, error: '' });
    if (!Object.values(recruits).some(Boolean) && !s.draftCard) return;
    if (
      requiredRecruitDiscards(s.view, s.city, recruits).some(
        (group) => group.units.filter((u) => replacements.includes(u.id)).length < group.count,
      )
    )
      return;
    try {
      this.patch({
        recruitPreview: this.query({
          kind: 'recruit',
          attackPirates: !!s.attackPirates,
          payment,
          draftCard: !!s.draftCard,
          city: s.city,
          units: recruits,
          replaced: replacements,
          ballcourts: !!s.ballcourts,
        }),
      });
    } catch (error) {
      this.patch({ error: String(error) });
    }
  }
  toggleRecruitDiscard(id: number) {
    const s = get(this.session);
    if (s.pending || s.playback) return;
    const group = requiredRecruitDiscards(s.view, s.city, s.recruits).find((g) =>
      g.units.some((u) => u.id === id),
    );
    if (!group) return;
    const selected = s.replacements.includes(id);
    if (!selected && group.units.filter((u) => s.replacements.includes(u.id)).length >= group.count) return;
    this.patch({
      replacements: selected ? s.replacements.filter((unit) => unit !== id) : [...s.replacements, id],
    });
    this.setRecruits(s.recruits);
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
  startInfluence(move: Move, actionPayment: Pile, rangePayment: Pile) {
    const s = get(this.session);
    if (s.pending || s.playback || !s.view?.canPlay) return;
    this.influenceAttemptPending = true;
    this.quotedInfluenceRangePayment = { ...rangePayment };
    this.submit(move, actionPayment);
  }
  researchPlan(action: Move | null): { eligible: boolean; affordable: boolean; combined?: boolean } {
    if (!action) return { eligible: false, affordable: false };
    try {
      return this.query({ kind: 'researchPlan', action });
    } catch {
      return { eligible: false, affordable: false };
    }
  }
  submitResearch(action: Move, buyFreeEducation: boolean) {
    const plan = this.researchPlan(action);
    this.quotedFreeEducation = plan.combined
      ? buyFreeEducation && plan.affordable
        ? { ideas: 1 }
        : {}
      : null;
    this.submit(action);
  }
  submit(move: Move, quotedPayment?: Pile) {
    const s = get(this.session);
    if (
      s.pending ||
      s.playback ||
      s.seat === undefined ||
      (!(s.view?.activePlayers ?? [s.view?.activePlayer]).includes(s.seat) &&
        !(
          s.view?.civilizationDraft &&
          typeof move === 'object' &&
          'ChooseCivilization' in move &&
          s.view.civilizations?.some((c) => c.name === move.ChooseCivilization)
        ))
    )
      return;
    this.patch({ pending: true, error: '' });
    this.submittedMove = move;
    this.quotedActionPayment =
      quotedPayment && Object.values(quotedPayment).some(Boolean) ? { ...quotedPayment } : null;
    if (!this.commands.move(JSON.stringify(move))) {
      this.handleError('The action could not be sent. Please try again.');
      return;
    }
    if (move === 'Undo') this.showUndoPreview();
    this.refreshTimer = setTimeout(() => {
      this.restoreUndoPreview();
      this.commands.fetchState();
      this.patch({ error: 'Waiting for the game to confirm your action…' });
    }, 8000);
  }
  private showUndoPreview() {
    const s = get(this.session);
    if (!s.pending || !s.game || !s.view || s.analysis) return;
    const snapshot = this.undoPreview.previous(s.game, s.view);
    if (!snapshot) return;
    const patch: Partial<Session> = {
      game: snapshot.game,
      view: snapshot.view,
      mode: 'overview',
      tilePanel: false,
      cardsOpen: false,
      wondersOpen: false,
      objectivesOpen: false,
      abilitiesOpen: false,
      abilityChoice: null,
      abilityCity: null,
      publicEffects: [],
      battles: [],
    };
    this.undoRestore = Object.fromEntries(Object.keys(patch).map((key) => [key, s[key as keyof Session]]));
    // Keep the authoritative raw state and pending lock until the host confirms.
    // No queries, automatic payments or other game moves use this preview.
    this.patch(patch);
  }
  private restoreUndoPreview() {
    if (!this.undoRestore) return;
    const restore = this.undoRestore;
    this.undoRestore = null;
    this.patch(restore);
  }
  playContextualCard(id: number, context: CardContext) {
    const s = get(this.session);
    if (s.pending || s.playback) return;
    const card = contextualCards(s.view, context).find((offer) => offer.card.id === id)?.card;
    if (!card?.action) return;
    this.cardContinuation = {
      id,
      mode: s.mode,
      city: s.city,
      cityTab: s.cityTab,
      selection: structuredClone(s.selection),
      collectVariant: s.collectVariant,
      ballcourts: s.ballcourts,
      selectedAdvance: s.selectedAdvance,
      happinessSteps: { ...s.happinessSteps },
      happinessVariant: s.happinessVariant,
      happinessCity: s.happinessCity,
      happinessLawgiver: s.happinessLawgiver,
    };
    this.submit(card.action, card.cost);
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
  researchReferenceView(player?: number): View | null {
    if (!this.engine || !this.raw) return null;
    // The browser only has the server-filtered state. Inspecting public research
    // must not change the viewer's seat, decision, selection, or available filter.
    const s = get(this.session);
    return JSON.parse(
      this.engine.webView(this.raw, player ?? s.seat ?? s.game?.current_player_index),
    ) as View;
  }
  notify(toast: string) {
    clearTimeout(this.timer);
    this.patch({ toast });
    this.timer = setTimeout(() => this.patch({ toast: '' }), 4000);
  }
  destroy() {
    this.undoPreview.clear();
    this.undoRestore = null;
    clearTimeout(this.battleTimer);
    this.destroyed = true;
    clearTimeout(this.playbackTimer);
    clearTimeout(this.effectTimer);
    clearTimeout(this.timer);
    clearTimeout(this.refreshTimer);
    this.chatOff();
    this.audio.destroy();
  }
}
