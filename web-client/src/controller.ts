import { activeInfluence, influenceKey, influenceMapPick, influencePaymentMatches } from './influence';
import { defaultCity, type CollectionPotential } from './default-city';
import { nextCollectionSelection } from './collection-yield';
import { get, writable } from 'svelte/store';
import { ChatController } from '@boardgamers/protocol/chat';
import type { ViewerCommands } from '@boardgamers/protocol/viewer';
import type { Bridge, Choice, Game, Move, RecruitSelection, Session, View, MapPick, Pile } from './types';
import { journal } from './model';
import { loadBridge } from './bridge';
import { readPreferences } from './preferences';
import { GameAudio, moveSound } from './audio';
import { CardDrawTracker } from './card-draws';
import { canMoveOnMap, instantMove, moveOrigins, passengerLandings } from './map-actions';
import { movementBonus } from './movement-bonus';
import { activeCityAbility, groupAbilities } from './abilities';
import { recapStart, sinceLastTurn, frameAt, frameEffects } from './playback';
import { groupPlaybackFrames, playbackSteps } from './replay-actions';
import { battleCues, battleCursor, BATTLE_DURATION, type BattleCue } from './battle-playback';
import { contextualCards, type CardContext } from './contextual-cards';
import { recruitDiscardSelection, requiredRecruitDiscards } from './recruit-discards';
import { happinessTargets } from './happiness';
import { UndoPreview } from './undo-preview';
import { fingerprint, isTrap, predictMove, predictUndo } from './prediction';
import {
  researchDecision,
  mapDecisionOptions,
  mapDecisionIndex,
  toggleDecisionSelection,
} from './decision-controls';
const WAITING = 'Waiting for the game to confirm your action…';
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
    confirmMoves: true,
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
  /** The state currently shown and queried: the server's, or a local prediction on top of it. */
  private raw = '';
  /** The latest state received from the host. */
  private confirmedRaw = '';
  private confirmedPrint: string | null = null;
  /**
   * Moves awaiting the server, oldest first. Only the head has been sent; the
   * rest wait for its confirmation so the host always receives moves in order.
   * `print` is the predicted state's fingerprint, or null when the result
   * depends on information this seat does not have.
   */
  private outbox: { move: Move; print: string | null; announced: boolean }[] = [];
  private predictions: boolean;
  private moveAnnounced = false;
  private undoPreview = new UndoPreview();
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
    options: { predict?: boolean } = {},
  ) {
    this.predictions = options.predict ?? true;
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
        next.influenceMode = false;
      }
      if (!next.influenceMode) {
        next.influencePosition = null;
        next.influenceTarget = null;
        next.influenceOrigin = null;
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
    // A rejected move invalidates its prediction and every move queued behind it.
    this.outbox = [];
    this.showConfirmed();
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
  /** The host's verdict on a sent move. A rejection withdraws its prediction at once. */
  moveResult({ move, ok, error }: { move?: unknown; ok: boolean; error?: string }) {
    const head = this.outbox[0];
    if (ok || !head || (typeof move === 'string' && move !== JSON.stringify(head.move))) return;
    this.handleError(error || 'The action could not be sent. Please try again.');
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
    if (raw === this.confirmedRaw && old.view) return;
    if (this.outbox.length && this.reconcile(raw)) return;
    clearTimeout(this.refreshTimer);
    this.confirmedRaw = raw;
    this.confirmedPrint = null;
    this.apply(raw);
  }
  private confirmedFingerprint() {
    return (this.confirmedPrint ??= fingerprint(this.confirmedRaw));
  }
  /** Matches a host state against the predicted moves. Returns true when it was handled. */
  private reconcile(raw: string) {
    const print = fingerprint(raw);
    // A resend of the state before the move (hidden piles are reshuffled per send).
    if (print === this.confirmedFingerprint()) return true;
    const head = this.outbox[0],
      base = this.confirmedRaw;
    this.confirmedRaw = raw;
    this.confirmedPrint = print;
    clearTimeout(this.refreshTimer);
    if (head.print === print) {
      // Exactly as predicted: nothing visible changes, nothing replays.
      this.outbox.shift();
      if (this.outbox.length) this.sendHead();
      else {
        this.raw = raw;
        this.markSeen();
        if (get(this.session).error === WAITING) this.patch({ error: '' });
      }
      return true;
    }
    if (head.print === null) {
      // The awaited result of a move that could not be predicted.
      this.outbox = [];
      this.submittedMove = head.move;
      this.moveAnnounced = head.announced;
      this.apply(raw);
      return true;
    }
    // The server disagrees with the prediction (or another player moved first).
    // Rewind to the last confirmed position quietly, then show the server's state
    // as a regular update so new draws, battles and recap effects are reported.
    this.outbox = [];
    if (this.raw !== base) this.apply(base, { quiet: true });
    this.apply(raw);
    this.notify('Game updated.');
    return true;
  }
  private apply(raw: string, { predicted = false, quiet = false } = {}) {
    if (!this.engine) return;
    if (quiet) {
      // Withdrawn moves must not continue with their quoted payments or card flows.
      this.quotedActionPayment = null;
      this.quotedInfluenceRangePayment = null;
      this.quotedFreeEducation = null;
      this.cardContinuation = null;
    }
    const old = get(this.session);
    const game = JSON.parse(raw) as Game;
    if (game.board_history) game.board_history = { ...game.board_history, frames: groupPlaybackFrames(game) };
    const changed = raw !== this.raw;
    this.raw = raw;
    this.moveCache.clear();
    const view = JSON.parse(this.engine.webView(raw, old.seat)) as View;
    this.undoPreview.remember(raw, game, view, old.seat);
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
    const influenceTarget = view.influence?.find((offer) => influenceKey(offer) === old.influenceTarget);
    this.patch({
      influenceTarget: influenceTarget ? old.influenceTarget : null,
      influencePosition: view.influence?.some((offer) => offer.position === old.influencePosition)
        ? old.influencePosition
        : null,
      influenceOrigin: influenceTarget?.origins?.some((o) => o.position === old.influenceOrigin)
        ? old.influenceOrigin
        : null,
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
    if (quiet) {
      // Effects of a withdrawn prediction must not linger on the board.
      this.dismissEffects(true);
      this.showBattles([]);
    }
    const announced = this.moveAnnounced;
    this.moveAnnounced = false;
    if (quiet) {
      /* Rewinding a prediction: the confirmed state was already announced. */
    } else if (bonusGranted) {
      this.audio.play('research');
      this.notify(`${bonus.source} · ${bonus.label}`);
    } else if (drawn.length) {
      this.audio.play('draw');
    } else if (old.pending && changed && !automaticPayment && !announced) {
      this.audio.play(moveSound(this.submittedMove));
      if (!this.influenceAttemptPending) this.notify('Game updated.');
    }
    if (!quiet && this.influenceAttemptPending && changed && !automaticPayment && !game.events?.length) {
      this.notify(
        game.successful_cultural_influence ? 'Cultural influence succeeded' : 'Cultural influence failed',
      );
      this.influenceAttemptPending = false;
    }
    this.submittedMove = null;
    this.afterPlaybackLoad(old, game, !predicted);
    this.migrateRazeSetting();
    if (automaticPayment) this.submit(automaticPayment);
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
  private afterPlaybackLoad(old: Session, game: Game, confirmed = true) {
    const s = get(this.session),
      frames = game.board_history?.frames ?? [];
    if (game.state === 'Finished' && s.playback?.automatic) {
      this.endPlayback();
      if (confirmed) this.markSeen();
      return;
    }
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
    // Predicted positions are marked seen once the server confirms them.
    if (confirmed) this.markSeen();
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
    this.settleToConfirmed();
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
      const index = mapDecisionIndex(s.view!.decision!, position, pick);
      // Pieces: open the clicked hex, and choose the clicked unit/building (or a lone candidate).
      if (s.view!.decision!.options.some((o) => o.mapTarget)) this.focusDecisionPosition(position);
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
    const influence = activeInfluence(s);
    if (influence) {
      const offer = influenceMapPick(influence.offers, position, pick);
      if (influence.targets.includes(position))
        this.patch({
          influencePosition: position,
          influenceTarget: offer ? influenceKey(offer) : null,
          influenceOrigin: null,
          error: '',
        });
      else if (influence.origins.includes(position)) this.patch({ influenceOrigin: position, error: '' });
      return;
    }
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
      const instant = s.confirmMoves ? null : instantMove(s.moveDestinations, position);
      this.chooseMoveDestination(position);
      if (instant) this.submit(instant.action);
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
  chooseInfluence() {
    const s = get(this.session);
    if (s.pending || !s.view?.canPlay || !s.view.influence?.length) return;
    this.closeActivity();
    this.patch({
      abilitiesOpen: true,
      abilityChoice: null,
      abilityCity: null,
      influenceMode: true,
      influencePosition: null,
      influenceTarget: null,
      influenceOrigin: null,
      mode: 'overview',
      tilePanel: false,
      seaRoutes: false,
      pirateSpawns: false,
      error: '',
    });
  }
  selectInfluenceTarget(key: string | null) {
    const s = get(this.session);
    const offer = s.view?.influence?.find((o) => influenceKey(o) === key);
    this.patch({
      influenceTarget: offer ? key : null,
      influencePosition: offer?.position ?? s.influencePosition ?? null,
      influenceOrigin: null,
      error: '',
    });
  }
  selectInfluenceOrigin(position: string) {
    this.patch({ influenceOrigin: position, error: '' });
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
    const city = s.view.cities.find((c) => c.position === s.city)!;
    const extraCapacity = Number(!!s.ballcourts && !!city.ballcourts);
    const capacity = city.capacity + extraCapacity;
    const selection = nextCollectionSelection(choice, s.selection, city, extraCapacity);
    if (!selection) {
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
    if ((move === 'Undo' && !s.view?.canUndo) || (move === 'Redo' && !s.view?.canRedo)) return;
    this.patch({ pending: true, error: '' });
    this.submittedMove = move;
    this.quotedActionPayment =
      quotedPayment && Object.values(quotedPayment).some(Boolean) ? { ...quotedPayment } : null;
    const predicted = this.predict(move, s);
    this.outbox.push({ move, print: predicted && fingerprint(predicted), announced: !!predicted });
    if (this.outbox.length === 1 && !this.sendHead()) return;
    // Show the result immediately. Queries and follow-up moves use the predicted
    // state; the host still validates every move and its state stays authoritative.
    if (predicted) this.apply(predicted, { predicted: true });
  }
  /** The visible result of `move`, when it does not depend on hidden information. */
  private predict(move: Move, s: Session): string | null {
    const engine = this.engine;
    if (
      !this.predictions ||
      !engine?.tryMove ||
      !engine.stripSecret ||
      s.analysis ||
      s.seat === undefined ||
      !s.game ||
      !s.view ||
      this.outbox.some((entry) => entry.print === null)
    )
      return null;
    try {
      if (move === 'Undo') {
        // Undo patches are stripped from the browser's state: restore the
        // position this seat was shown one step earlier on the same branch.
        const previous = this.undoPreview.previous(s.game, s.view);
        const undone = previous && predictUndo(previous, this.raw);
        return undone ? engine.stripSecret(undone, s.seat) : null;
      }
      const prediction = predictMove(
        { tryMove: engine.tryMove, stripSecret: engine.stripSecret },
        this.raw,
        move,
        s.seat,
      );
      return 'raw' in prediction ? prediction.raw : null;
    } catch (error) {
      // A trapped engine must not be exercised for optional work again.
      if (isTrap(error)) this.predictions = false;
      return null;
    }
  }
  private sendHead() {
    const head = this.outbox[0];
    if (!this.commands.move(JSON.stringify(head.move))) {
      this.handleError('The action could not be sent. Please try again.');
      return false;
    }
    clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(() => {
      // Show the confirmed position again and wait, as for an unpredictable move.
      this.settleToConfirmed();
      this.commands.fetchState();
      this.patch({ error: WAITING });
    }, 8000);
    return true;
  }
  /**
   * Withdraws every prediction: unsent moves are dropped, and the sent move is
   * still awaited, but any new server state now counts as its result.
   */
  private settleToConfirmed() {
    clearTimeout(this.refreshTimer);
    const head = this.outbox[0];
    this.outbox = head ? [{ ...head, print: null }] : [];
    if (this.outbox.length > 0 || this.raw !== this.confirmedRaw) {
      this.showConfirmed();
      if (head) this.patch({ pending: true });
    }
  }
  /** Rewinds the display to the last state received from the host, without effects. */
  private showConfirmed() {
    if (!this.confirmedRaw || this.raw === this.confirmedRaw) return;
    this.apply(this.confirmedRaw, { quiet: true });
    // The withdrawn move's "Game updated." no longer applies.
    clearTimeout(this.timer);
    this.patch({ toast: '' });
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
    this.outbox = [];
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
