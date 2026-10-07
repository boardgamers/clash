<script lang="ts">
  import { formatPoints } from './score';
  import { currentLocale, translateText } from './localization';
  const translate = $derived((value: string) => translateText(value, $currentLocale));
  import CollectionBonusHints from './CollectionBonusHints.svelte';
  import BarbarianIcon from './BarbarianIcon.svelte';
  import CitySiteIcon from './CitySiteIcon.svelte';
  import PlaybackPanel from './PlaybackPanel.svelte';
  import CardReferenceDialog from './CardReferenceDialog.svelte';
  import { cardReferences, type CardReference, type CardRule } from './card-reference';
  import InfluenceFlow from './InfluenceFlow.svelte';
  import BattlePlayback from './BattlePlayback.svelte';
  import PublicEffects from './PublicEffects.svelte';
  import EventMarkers from './EventMarkers.svelte';
  import CivilizationPicker from './CivilizationPicker.svelte';
  import TilePanel from './TilePanel.svelte';
  import { onMount } from 'svelte';
  import {
    Skull,
    Landmark,
    Compass,
    Wheat,
    Trees,
    Mountain,
    Lightbulb,
    Coins,
    Smile,
    Drama,
    Link,
    BookOpen,
    ScrollText,
    MessageCircle,
    ArrowRight,
    ArrowLeft,
    Check,
    Plus,
    Minus,
    Maximize,
    Minimize,
    Layers,
    Trophy,
    Flag,
    X,
    ChevronRight,
    Hourglass,
    Undo2,
    Redo2,
    GraduationCap,
    Target,
    MapPin,
    Hammer,
    Footprints,
    Users,
    Swords,
    Volume2,
    VolumeX,
    Eye,
    Shapes,
    RotateCw,
    Zap,
    Ship,
    Frown,
    Meh,
    Crown,
    Sparkles,
    ShieldCheck,
    CircleSlash,
    TriangleAlert,
    Hexagon,
    History,
    Info,
  } from 'lucide-svelte';
  import { mountChat } from '@boardgamers/protocol/chat/dom';
  import { World } from './board';
  import { militarySummary } from './strategy';
  import { positionStrategyKey, positionMapGuide } from './strategy-key';
  import ResearchTree from './ResearchTree.svelte';
  import { researchDecision, mapDecisionOptions } from './decision-controls';
  import CityPanel from './CityPanel.svelte';
  import HappinessPanel from './HappinessPanel.svelte';
  import SettlerPanel from './SettlerPanel.svelte';
  import ExplorationPanel from './ExplorationPanel.svelte';
  import ScoreDialog from './ScoreDialog.svelte';
  import WondersDialog from './WondersDialog.svelte';
  import HowToPlay from './HowToPlay.svelte';
  import CardReveal from './CardReveal.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';
  import ObjectiveCondition from './ObjectiveCondition.svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import { civilizationAccent } from './civilization-theme';
  import CityFacts from './CityFacts.svelte';
  import CollectionCapacity from './CollectionCapacity.svelte';
  import ContextualCards from './ContextualCards.svelte';
  import DecisionPanel from './DecisionPanel.svelte';
  import CombatJournal from './CombatJournal.svelte';
  import { combatJournal } from './combat-journal';
  import ActionCardsDialog from './ActionCardsDialog.svelte';
  import { activeCityAbility } from './abilities';
  import { mobilePanels } from './mobile-panels';
  import AbilitiesPanel from './AbilitiesPanel.svelte';
  import type { Controller } from './controller';
  import type { Resource, JournalEntry, View } from './types';
  import { researchReferences, type ResearchReference } from './research-links';
  import { resources, resourceNames, playerColor, playerSymbol } from './types';
  import { journal, pileText } from './model';
  import {
    collectionYield,
    collectionBonusLabel,
    collectionBonusIndicators,
    collectionStorageWaste,
  } from './collection-yield';
  import CollectionIndicators from './CollectionIndicators.svelte';
  import { movementBonus } from './movement-bonus';
  import { sinceLastTurn } from './playback';
  import { ageCount, ageLabel } from './game-length';
  import { roundProgress, personalRoundLabel } from './round-progress';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const lastTurn = $derived($session.game ? sinceLastTurn($session.game, $session.seat) : null);
  let boardHost: HTMLDivElement;
  let world: World;
  let boardError = $state('');
  let fullscreenEnabled = $state(false);
  let fullscreen = $state(false);
  let confirmEnd = $state(false);
  let cardReference = $state<CardRule | null>(null);
  const journalCards = $derived(cardReferences($session.view));
  function showJournalCard(reference: CardReference) {
    cardReference = reference.card;
  }
  let researchReference = $state<{ target: ResearchReference; view: View } | null>(null);
  let seaTooltipDismissed = $state(false);
  let mapMinimized = $state(false);
  let mapModalMinimized = $state(false);
  const panelVisibilityKey = $derived(
    JSON.stringify([
      $session.seat,
      $session.game?.log_index,
      $session.mode,
      $session.view?.decision?.name,
      !!$session.view?.explorationDecision,
      $session.help,
      $session.cardsOpen,
      $session.wondersOpen,
      $session.objectivesOpen,
      $session.scorePlayer,
      $session.abilitiesOpen,
      $session.activityOpen,
      $session.tab,
    ]),
  );
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
    captives: Link,
  };
  const journalTokenIcons = {
    ...icons,
    action: Zap,
    wonder: Landmark,
    objective: Target,
    card: Layers,
    research: GraduationCap,
    city: Landmark,
    unit: Users,
    happy: Smile,
    neutral: Meh,
    angry: Frown,
    event: Hourglass,
  };
  const journalIcons = {
    setup: Landmark,
    collect: Wheat,
    research: GraduationCap,
    objective: Target,
    'end-turn': Flag,
    move: Footprints,
    build: Hammer,
    recruit: Users,
    combat: Swords,
    card: Layers,
    event: ScrollText,
  };
  let current = $derived($session.game?.players.find((p) => p.id === $session.seat));
  const visibleResources = $derived(
    resources.filter((r) => r !== 'captives' || !!current?.resources?.captives),
  );
  let city = $derived($session.view?.cities.find((c) => c.position === $session.city));
  const collectionVariant = $derived(
    ($session.view?.collectActions ?? []).find(
      (a) => JSON.stringify(a.value) === JSON.stringify($session.collectVariant),
    ),
  );

  const collectionFree = $derived(collectionVariant?.free ?? false);
  const collectionCost = $derived(collectionVariant?.payment ?? {});
  const collectionCostLabel = $derived(
    [
      Object.values(collectionCost).some(Boolean) ? `Pay ${pileText(collectionCost)}` : '',
      collectionFree ? '0 actions' : '1 action',
    ]
      .filter(Boolean)
      .join(' · '),
  );
  let identity = $derived($session.view?.players.find((p) => p.index === $session.seat));
  const playerTurns = $derived(
    roundProgress(
      $session.playback || $session.view?.civilizationDraft ? null : $session.game,
      $session.view?.players ?? [],
    ),
  );
  const roundHint = $derived(
    personalRoundLabel(
      $session.game?.round ?? 1,
      playerTurns.find((p) => p.player.index === $session.seat)?.status,
    ),
  );
  const activePlayers = $derived(
    $session.game?.state === 'Finished' || $session.playback
      ? []
      : ($session.view?.activePlayers ?? ($session.view ? [$session.view.activePlayer] : [])),
  );
  const choosingPlayer = $derived(
    !!$session.view?.civilizationDraft || !!$session.game?.events?.length || ($session.game?.round ?? 0) > 3,
  );
  const waiting = $derived(
    !$session.playback && $session.view?.waitingFor?.player !== $session.seat
      ? $session.view?.waitingFor
      : null,
  );
  let log = $derived(
    $session.game ? combatJournal(journal($session.game, $session.view ?? undefined)).reverse() : [],
  );
  const researchCatalog = $derived(
    $session.view && ($session.view.advances.length ? $session.view : controller.researchReferenceView()),
  );
  function showJournalResearch(target: ResearchReference) {
    const view = controller.researchReferenceView(target.player);
    if (view) researchReference = { target, view };
  }
  function showJournalObjective(target: { name: string; player: number }) {
    controller.patch({ scorePlayer: target.player, scoreTab: 'objectives', scoreObjective: target.name });
  }
  const coordinateInteraction = {
    onCoordinate: (position: string | null) => world?.highlightCoordinate(position),
    onLocate: (position: string) => {
      world?.locateCoordinate(position);
      controller.closeActivity();
    },
  };
  let mapPositions = $derived(new Set($session.game?.map.tiles.map(([position]) => position) ?? []));
  let totalActions = $derived($session.game?.actions_left ?? 0);
  let readyToEnd = $derived(!!$session.view?.canEndTurn && totalActions === 0);
  const tradeWarning = $derived($session.view?.endTurnTradeWarning);
  $effect(() => {
    if (!$session.view?.canEndTurn) confirmEnd = false;
  });
  let collectAvailable = $derived($session.view?.cities.some((c) => !c.reason && c.choices.length > 0));
  let researchAvailable = $derived($session.view?.advances.some((a) => !!a.action));
  let researchChoice = $derived(researchDecision($session.view));
  let mapDecision = $derived(mapDecisionOptions($session.view?.decision).length > 0);
  let cityActionsAvailable = $derived(
    $session.view?.cityActions.some(
      (c) =>
        c.buildings.some((b) => b.choices.length > 0) ||
        c.recruits.some((r) => !r.reason && r.available > 0) ||
        c.leaders?.some((l) => l.reason === null) ||
        c.happiness.some((h) => !!h.action),
    ),
  );
  let settlersAvailable = $derived(
    !!$session.view?.stopMovement ||
      (!!$session.view?.canPlay &&
        totalActions > 0 &&
        (!!$session.view?.units?.length || !!$session.view?.nomadCities?.length)),
  );
  let abilitiesAvailable = $derived(
    !!$session.view?.specialActions?.length ||
      !!$session.view?.influence?.length ||
      !!($session.view && controller.shogunateDraftOffers().length),
  );
  let objectiveDecision = $derived($session.view?.objectiveDecision);
  let choiceDecision = $derived($session.view?.choiceDecision);
  let activeMovementBonus = $derived(movementBonus($session.game));
  let actionTitle = $derived(
    $session.pending
      ? 'Confirming…'
      : waiting
        ? `Waiting for ${$session.view?.players.find((p) => p.index === waiting.player)?.civilization ?? 'another player'}`
        : $session.seat === undefined
          ? 'Spectating'
          : $session.game?.state === 'Finished'
            ? 'Game over'
            : $session.view?.decision
              ? $session.view.decision.endOfAge
                ? 'End of age'
                : $session.view.decision.name
              : $session.view?.explorationDecision
                ? 'Place explored terrain'
                : choiceDecision
                  ? 'Choose a bonus'
                  : objectiveDecision
                    ? 'Objective available'
                    : $session.view?.stopMovement
                      ? activeMovementBonus
                        ? `${activeMovementBonus.source} moves`
                        : 'Moving units'
                      : readyToEnd
                        ? 'Ready to end turn'
                        : $session.view?.canPlay
                          ? 'Your turn'
                          : `${$session.view?.players.find((p) => p.index === $session.view?.activePlayer)?.civilization ?? 'Opponent'}’s turn`,
  );
  onMount(() => {
    fullscreenEnabled = document.fullscreenEnabled;
    const updateFullscreen = () => {
      fullscreen = !!document.fullscreenElement;
    };
    updateFullscreen();
    document.addEventListener('fullscreenchange', updateFullscreen);
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || document.fullscreenElement || document.querySelector('dialog[open]'))
        return;
      if ($session.playback) {
        controller.endPlayback();
        return;
      }
      if ($session.seaRoutes) controller.showSeaRoutes(false);
      if ($session.cardDraws.length) controller.patch({ cardDraws: [] });
      dismissMapDetails();
    };
    document.addEventListener('keydown', escape);
    let off: (() => void) | undefined;
    try {
      world = new World(
        boardHost,
        (p, pick) => {
          if (mapModalMinimized) return;
          if (
            !mapMinimized &&
            ($session.activityOpen || ($session.abilitiesOpen && !activeCityAbility($session)) || confirmEnd)
          )
            dismissMapDetails();
          controller.selectTile(p, pick);
        },
        () => controller.audio.play('hover'),
        dismissMapDetails,
      );
      off = session.subscribe((s) => world.update(s));
    } catch (e) {
      boardError =
        'The 3D map could not start on this device. You can still select your city and collect resources with the controls.';
    }
    return () => {
      document.removeEventListener('keydown', escape);
      document.removeEventListener('fullscreenchange', updateFullscreen);
      off?.();
      world?.destroy();
    };
  });
  export function renderThumbnail(width: number, height: number) {
    return world?.thumbnail(width, height) ?? null;
  }
  async function toggleFullscreen() {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch {
      controller.patch({ error: 'Fullscreen is unavailable in this browser.' });
    }
  }
  function chatPanel(node: HTMLElement) {
    const panel = mountChat(node, {
      chat: controller.chat,
      openPlayer: controller.commands.openPlayer,
      labels: {
        title: 'Chat',
        empty: 'No messages yet.',
        placeholder: 'Message…',
      },
    });
    controller.chat.setOpen($session.activityOpen && $session.tab === 'chat');
    return { destroy: () => panel.destroy() };
  }
  function showDialog(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
  function closeHelp() {
    controller.patch({ help: false });
  }
  function closeAction() {
    confirmEnd = false;
    controller.patch({
      mode: 'overview',
      tilePanel: false,
      collectionTile: null,
      moveTarget: null,
      moveDestination: null,
      selection: [],
      preview: null,
      error: '',
    });
  }
  function dismissMapDetails() {
    if ($session.pending || mapMinimized) return;
    world?.clearCoordinate();
    controller.closeActivity();
    controller.patch({ focus: $session.city, abilitiesOpen: false, seaRouteStart: null });
    closeAction();
  }
  function openResearch() {
    confirmEnd = false;
    controller.closeActivity();
    controller.patch({
      mode: 'research',
      tilePanel: false,
      selectedAdvance: null,
      error: '',
      abilitiesOpen: false,
    });
  }
  function toggleActivity(tab: 'journal' | 'chat') {
    confirmEnd = false;
    if ($session.activityOpen && $session.tab === tab) controller.closeActivity();
    else controller.setTab(tab);
  }
  function closeObjectives() {
    controller.patch({ objectivesOpen: false });
  }
</script>

{#snippet journalRow(entry: JournalEntry, outcome = false)}
  {@const EntryIcon = journalIcons[entry.kind]}
  {@const research = researchReferences(researchCatalog, entry.player)}
  {@const objectives =
    $session.view?.players
      .find((p) => p.index === entry.player)
      ?.completedObjectives?.map((o) => ({ name: o.name, player: entry.player! })) ?? []}
  <article
    class:journal-event={!!entry.event}
    class:journal-outcome={outcome}
    class:journal-combat={!!entry.combat}
  >
    <span
      class="journal-symbol"
      aria-hidden="true"
      style={`--player:${playerColor(entry.player ?? 0, $session.colorBlind, $session.playerColors)}`}
    >
      {#if entry.civilization}<CivilizationEmblem
          civilization={entry.civilization}
          size={23}
        />{:else}<EntryIcon size={17} />{/if}
    </span>
    <div class="journal-heading">
      {#if entry.civilization}<strong>{entry.civilization}</strong>{/if}
      {#if !outcome}<span class:journal-event-title={!!entry.event}
          >{#if !entry.event}<EntryIcon size={13} aria-hidden="true" />{/if}<ResourceText
            text={entry.title}
            {research}
            onResearch={showJournalResearch}
            cards={journalCards}
            onCard={showJournalCard}
            positions={mapPositions}
            {...coordinateInteraction}
          /></span
        >{/if}
    </div>
    {#if entry.combat}<CombatJournal
        combat={entry.combat}
        {researchCatalog}
        onResearch={showJournalResearch}
      />
      {#each entry.combat.outcomes as effect}{@render journalRow(effect, true)}{/each}
    {/if}
    {#if entry.tokens.length}<div class="journal-deltas">
        {#each entry.tokens as token}{@const TokenIcon = journalTokenIcons[token.icon]}
          <span
            class="journal-delta"
            class:gain={token.tone === 'gain'}
            class:loss={token.tone === 'loss'}
            title={token.description}
            aria-label={token.description}
          >
            {#if token.value}<b aria-hidden="true">{token.value}</b>{/if}<TokenIcon
              size={14}
              aria-hidden="true"
            />
            <span class:sr-only={token.compact}
              ><ResourceText
                text={token.label}
                {research}
                onResearch={showJournalResearch}
                cards={journalCards}
                onCard={showJournalCard}
                positions={mapPositions}
                {...coordinateInteraction}
              /></span
            >
          </span>
        {/each}
      </div>{/if}
    {#if entry.notes.length}<p class="journal-notes">
        <ResourceText
          text={entry.notes.join(', ')}
          {objectives}
          onObjective={showJournalObjective}
          {research}
          onResearch={showJournalResearch}
          cards={journalCards}
          onCard={showJournalCard}
          positions={mapPositions}
          {...coordinateInteraction}
        />
      </p>{/if}
    {#if entry.collection}
      {#each entry.collection.effects as effect}
        <div class="collection-included">
          <span
            ><GraduationCap size={13} aria-hidden="true" /><ResourceText
              text={effect.source}
              {research}
              onResearch={showJournalResearch}
              cards={journalCards}
              onCard={showJournalCard}
            /></span
          >
          {#each effect.tokens as token}{@const Icon = journalTokenIcons[token.icon]}
            <span title={token.description} aria-label={token.description}
              >{token.value}<Icon size={13} aria-hidden="true" /></span
            >
          {/each}<small>included</small>
        </div>
      {/each}
      <details class="collection-sources">
        <summary aria-label={`Collection sources for ${entry.title.split(' · ')[1]}`}>
          {#if entry.collection.city}<CollectionCapacity {...entry.collection.city} /><span aria-hidden="true"
              >·</span
            >{/if}
          <span
            ><Hexagon size={13} aria-hidden="true" />{entry.collection.tiles.reduce(
              (sum, tile) => sum + tile.times,
              0,
            )} collected</span
          ><ChevronRight size={13} />
        </summary>
        {#if entry.collection.city}<p class="collection-structures">
            {entry.collection.city.structures.join(' + ')}
          </p>{/if}
        {#each entry.collection.tiles as tile}
          <div class="collection-source">
            <ResourceText text={tile.position} positions={mapPositions} {...coordinateInteraction} />
            <ResourceAmount
              pile={Object.fromEntries(Object.entries(tile.pile).map(([r, n]) => [r, n! * tile.times]))}
            />
          </div>
        {/each}
      </details>
    {/if}

    {#if entry.event}
      {#each entry.event.outcomes as effect (effect.id)}
        {@render journalRow(effect, true)}
      {/each}
      {#each entry.event.explanations as explanation}
        {@const faction = $session.game?.players.find((p) => p.id === explanation.player)?.civilization}
        <div class="event-explanation">
          {#if explanation.protected}<ShieldCheck size={15} aria-hidden="true" />{:else}<CircleSlash
              size={15}
              aria-hidden="true"
            />{/if}
          <div>
            {#if faction}<strong>{faction}</strong>{/if}<span
              ><ResourceText
                text={explanation.text}
                research={researchReferences(researchCatalog, explanation.player)}
                onResearch={showJournalResearch}
                cards={journalCards}
                onCard={showJournalCard}
              /></span
            >
          </div>
        </div>
      {/each}
      {#if entry.event.pending}<p class="event-pending">
          <Hourglass size={14} /><ResourceText
            text={entry.event.pending}
            {research}
            onResearch={showJournalResearch}
            cards={journalCards}
            onCard={showJournalCard}
          />
        </p>{/if}
      {#if entry.event.info}
        <details class="event-rules">
          <summary aria-label={`${entry.title} rules`}
            ><BookOpen size={13} /> Rules <ChevronRight size={13} /></summary
          >
          {#each entry.event.info.rules as rule}
            <p>
              <ResourceText
                text={rule}
                {research}
                onResearch={showJournalResearch}
                cards={journalCards}
                onCard={showJournalCard}
                positions={mapPositions}
                {...coordinateInteraction}
              />
            </p>
          {/each}
          {#if entry.event.info.baseEffect && entry.event.info.protectionAdvance}
            <p>Protection does not prevent the base effect.</p>
          {/if}
        </details>
      {/if}
    {/if}
  </article>
{/snippet}

<div
  use:mobilePanels={{
    key: panelVisibilityKey,
    onChange: (minimized, modal) => {
      mapMinimized = minimized;
      mapModalMinimized = modal;
    },
  }}
  class:dark={$session.dark}
  class:colorblind={$session.colorBlind}
  class:spectating={$session.seat === undefined}
  class:board-interacting={$session.mode === 'collect' ||
    $session.mode === 'happiness' ||
    $session.mode === 'settlers' ||
    $session.tilePanel ||
    !!activeCityAbility($session) ||
    !!$session.view?.decision?.tacticsSelection ||
    !!$session.view?.explorationDecision ||
    mapMinimized ||
    mapDecision}
  class:map-decision={mapDecision}
  class:activity-open={$session.activityOpen}
  class="game-shell"
  class:playing-back={!!$session.playback}
  class:civilization-setup={!!$session.view?.civilizationDraft}
  style:--civilization-accent={civilizationAccent(identity?.civilization)}
>
  <header class="masthead">
    <div class="brand-block">
      {#if $session.analysis}<span class="analysis-label">Analysis · Simulation</span>{/if}
      <button
        class="brand"
        onclick={() => controller.commands.openBoardgame()}
        aria-label="About Clash of Cultures"
        ><span class="brand-name" translate="no">Clash <i>of</i> Cultures</span><span class="mobile-era"
          >{#if $session.game?.options?.variant === 'Builder'}Builder ·
          {/if}{#if $session.view?.civilizationDraft}Civilization draft{:else}Age {ageLabel(
              $session.playback?.frame?.age ?? $session.game?.age ?? 1,
            )} · {($session.playback?.frame?.round ?? $session.game?.round ?? 1) > 3
              ? 'End of age'
              : `Round ${$session.playback?.frame?.round ?? $session.game?.round ?? 1}/3`}{/if}</span
        >{#if roundHint}<span class="mobile-turn-status" title="Your turn in the current round"
            >{roundHint}</span
          >{/if}</button
      >
      <p class="game-credits">
        <span title="Game design: Christian Marcussen">Christian Marcussen</span>
        <span aria-hidden="true">·</span>
        <a
          href="https://wizkids.com/clash-of-cultures-monumental-edition/"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Publisher: WizKids"
          title="Published by WizKids">WizKids</a
        >
      </p>
    </div>
    <div
      class="age-track"
      role="group"
      aria-label={`Age ${$session.playback?.frame?.age ?? $session.game?.age ?? 1} of ${ageCount($session.game)}`}
    >
      {#each Array.from({ length: ageCount($session.game) }, (_, i) => i + 1) as age}
        <span
          class="age-step"
          class:current={age === ($session.playback?.frame?.age ?? $session.game?.age ?? 1)}
          class:past={age < ($session.playback?.frame?.age ?? $session.game?.age ?? 1)}
          title={`Age ${age} of ${ageCount($session.game)}`}
        >
          {ageLabel(age)}
        </span>
      {/each}
      <small class="round-label"
        >{($session.playback?.frame?.round ?? $session.game?.round ?? 1) > 3
          ? 'End of age'
          : `Round ${$session.playback?.frame?.round ?? $session.game?.round ?? 1}/3`}{#if roundHint}<span
            class="round-personal"
          >
            · {roundHint}</span
          >{/if}</small
      >
    </div>
    <nav class="header-actions">
      {#if $session.seat !== undefined}
        <button
          class="text-button cards-button"
          data-tutorial="cards"
          aria-label={`Action cards: ${$session.view?.actionCards?.length ?? 0}`}
          onclick={() => controller.patch({ cardsOpen: true })}
          ><Layers size={17} /><span>Cards</span><span class="card-count"
            >{$session.view?.actionCards?.length ?? 0}</span
          ></button
        >
        <button
          class="text-button wonders-button"
          aria-label={`Wonders: ${$session.view?.wonderCards?.length ?? 0} ${$session.view?.wonderCards?.length === 1 ? 'card' : 'cards'}`}
          onclick={() => controller.patch({ wondersOpen: true, cardDraws: [] })}
          ><Landmark size={17} /> Wonders
          <span class="card-count">{$session.view?.wonderCards?.length ?? 0}</span></button
        >
        <button
          class="text-button objectives-button"
          aria-label={`Objectives: ${$session.view?.objectiveCards?.length ?? 0} ${$session.view?.objectiveCards?.length === 1 ? 'card' : 'cards'}`}
          onclick={() => controller.patch({ objectivesOpen: true })}
          ><Target size={17} /> Objectives
          <span class="card-count">{$session.view?.objectiveCards?.length ?? 0}</span></button
        >
      {/if}
      <button class="text-button" aria-label="How to play" onclick={() => controller.patch({ help: true })}
        ><BookOpen size={17} /> How to play</button
      >
    </nav>
    {#if current}
      <section
        class="resource-bar"
        aria-label="Your resources"
        style={`--resource-count:${visibleResources.length}`}
      >
        {#each visibleResources as resource}{@const Icon = icons[resource]}{@const limit = [
            'food',
            'wood',
            'ore',
            'ideas',
            'gold',
          ].includes(resource)
            ? current.resource_limit?.[resource]
            : undefined}
          <div
            class="resource"
            role="img"
            aria-label={`${translate(resourceNames[resource])}: ${current.resources?.[resource] ?? 0}${limit !== undefined ? translate(`, storage limit ${limit}`) : ''}`}
            title={`${translate(resourceNames[resource])}${limit !== undefined ? translate(` · Storage limit ${limit}`) : ''}`}
          >
            <span class="resource-icon {resource}"><Icon size={20} strokeWidth={1.8} /></span><span
              ><strong
                >{current.resources?.[resource] ?? 0}{#if limit !== undefined}<em> / {limit}</em>{/if}</strong
              ></span
            >
          </div>{/each}
      </section>
    {/if}
  </header>
  <main class="play-layout">
    <section class="map-section" aria-label="The civilization map">
      <div class="map-world" bind:this={boardHost}></div>
      <div class="map-vignette"></div>
      <div class="player-list" aria-label="Civilizations in turn order">
        {#each playerTurns as { player, order, status }}
          {@const turnLabel = activePlayers.includes(player.index)
            ? choosingPlayer
              ? 'Choosing'
              : player.index === $session.seat
                ? 'Your turn'
                : 'Current turn'
            : null}
          {@const progressLabel =
            status === 'current' ? 'Turn in progress' : status === 'left' ? 'Left game' : null}
          <button
            class="player-card"
            class:active={!!turnLabel}
            style={`--player:${playerColor(player.index, $session.colorBlind, $session.playerColors)}`}
            title={`Inspect ${player.civilization} (${player.index === $session.seat ? 'You' : player.name}): advances and victory points`}
            aria-label={[
              `${translate(player.civilization)}: ${formatPoints(player.score)} ${translate('victory points')}`,
              player.index === $session.seat ? translate('You') : player.name,
              order ? translate(`Turn order ${order}.`).replace(/\.$/u, '') : '',
              turnLabel ? translate(turnLabel) : '',
              progressLabel ? translate(progressLabel) : '',
            ]
              .filter(Boolean)
              .join('. ')}
            onclick={() => controller.patch({ scorePlayer: player.index })}
            onmouseenter={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              controller.commands.hoverPlayer(player.index, {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
              });
            }}
            onmouseleave={() => controller.commands.leavePlayer()}
            onfocus={() => {}}
            onblur={() => controller.commands.leavePlayer()}
          >
            <span class="civilization-watermark" aria-hidden="true"
              ><CivilizationEmblem civilization={player.civilization} size={64} /></span
            >
            <span class="player-emblem"
              >{#if $session.colorBlind}<span class="ownership-symbol" aria-hidden="true"
                  >{playerSymbol(player.index, $session.playerSymbols)}</span
                >{:else}<CivilizationEmblem
                  civilization={player.civilization}
                  size={24}
                />{/if}{#if order}<span class="player-order" title={`Turn order: ${order}`} aria-hidden="true"
                  >{order}</span
                >{/if}</span
            ><span class="player-info"
              ><strong>{player.civilization}</strong><small
                >{player.index === $session.seat
                  ? 'You'
                  : player.name}{#if $session.playerBadges?.[player.index]}<img
                    class="supporter-badge"
                    src={$session.playerBadges[player.index]!.url}
                    alt={$session.playerBadges[player.index]!.label}
                    title={$session.playerBadges[player.index]!.label}
                  />{/if}</small
              ><span class="player-events"><EventMarkers remaining={player.eventTokens} /></span
              >{#if turnLabel || progressLabel}<span class="player-turn" class:round-inactive={!turnLabel}
                  >{#if turnLabel || status === 'current'}<ArrowRight
                      size={12}
                      aria-hidden="true"
                    />{/if}{turnLabel ?? progressLabel}</span
                >{/if}{#if $session.strategyMap}{@const forces = militarySummary(
                  ($session.playback?.frame?.players ?? $session.game?.players ?? []).find(
                    (p) => p.id === player.index,
                  ) ?? { id: player.index, civilization: player.civilization },
                )}<span
                  class="strategy-player-forces"
                  aria-label={`${forces.army} army units, ${forces.ships} ships${forces.aboard ? `, ${forces.aboard} army aboard` : ''}`}
                  title="Army includes leaders and embarked troops; settlers are separate"
                  ><Swords size={13} />{forces.army}<Ship size={13} />{forces.ships}</span
                >{/if}</span
            ><span class="player-score">{formatPoints(player.score)}<Trophy size={10} /></span>
          </button>{/each}
      </div>
      {#if !$session.game}<div class="map-loading">
          <Compass size={30} />
          <h2>Loading game…</h2>
        </div>{/if}
      {#if boardError}<div class="map-warning" role="alert">{boardError}</div>{/if}
      {#if $session.tilePanel && !$session.seaRoutes && $session.mode === 'overview' && !$session.view?.decision && !choiceDecision && !objectiveDecision && !$session.view?.explorationDecision}
        <TilePanel {controller} onHighlight={(p) => world?.highlightCoordinate(p)} />
      {/if}
      <div class="map-controls">
        <div class="sr-only" id="sea-route-help" aria-live="polite">
          {$session.seaRouteStart
            ? `Sea routes from ${$session.seaRouteStart}. Arrow keys choose another sea tile.`
            : 'Toggle all sea routes. Hover a sea tile to preview its routes. When enabled, select a sea tile or use arrow keys to focus its routes. Dashed shortcuts require Navigation.'}
        </div>
        <div class="map-control-help">
          <button
            class:active={$session.seaRoutes && $session.mode === 'overview'}
            aria-label="Show all sea routes"
            aria-pressed={$session.seaRoutes && $session.mode === 'overview'}
            aria-describedby="sea-route-tooltip sea-route-help"
            onpointerenter={() => (seaTooltipDismissed = false)}
            onfocus={() => (seaTooltipDismissed = false)}
            onkeydown={(event) => {
              if (event.key === 'Escape') seaTooltipDismissed = true;
              if ($session.seaRoutes && ['ArrowRight', 'ArrowLeft'].includes(event.key)) {
                event.preventDefault();
                controller.nextSeaRoute(event.key === 'ArrowRight' ? 1 : -1);
              }
            }}
            onclick={() => controller.showSeaRoutes(!($session.seaRoutes && $session.mode === 'overview'))}
            ><Ship size={18} /></button
          >
          <div
            id="sea-route-tooltip"
            role="tooltip"
            class="map-tooltip"
            class:dismissed={seaTooltipDismissed}
          >
            <strong>Sea-route guide</strong>
            <div><i class="route-line" aria-hidden="true"></i>Connected sea tiles</div>
            <div><i class="route-line dashed" aria-hidden="true"></i>Shortcut requiring Navigation</div>
            <p>Hover a sea tile to preview. Click here to show all routes.</p>
          </div>
        </div>
        <div class="map-control-help">
          <button
            aria-label="Show pirate spawn positions"
            title="Pirate spawn positions"
            disabled={!!$session.playback ||
              !!$session.view?.decision ||
              !!$session.view?.choiceDecision ||
              !!$session.view?.objectiveDecision ||
              !!$session.view?.explorationDecision ||
              !$session.view?.pirateSpawns}
            class:active={$session.pirateSpawns &&
              $session.threatGuide !== 'barbarians' &&
              $session.mode === 'overview'}
            aria-pressed={$session.pirateSpawns &&
              $session.threatGuide !== 'barbarians' &&
              $session.mode === 'overview'}
            onclick={() =>
              controller.showPirateSpawns(!($session.pirateSpawns && $session.threatGuide !== 'barbarians'))}
            ><Skull size={18} /></button
          >
          <button
            title="Barbarian spawn and movement guide"
            aria-label="Show barbarian spawn and movement"
            class:active={$session.pirateSpawns &&
              $session.threatGuide === 'barbarians' &&
              $session.mode === 'overview'}
            aria-pressed={$session.pirateSpawns &&
              $session.threatGuide === 'barbarians' &&
              $session.mode === 'overview'}
            disabled={!!$session.playback ||
              !!$session.view?.decision ||
              !!$session.view?.choiceDecision ||
              !!$session.view?.explorationDecision}
            onclick={() =>
              controller.showPirateSpawns(
                !($session.pirateSpawns && $session.threatGuide === 'barbarians'),
                'barbarians',
              )}><BarbarianIcon /></button
          >
          {#if $session.pirateSpawns && $session.mode === 'overview' && !$session.view?.decision && !$session.view?.choiceDecision && !$session.view?.objectiveDecision && !$session.view?.explorationDecision && !$session.abilitiesOpen && !$session.playback}
            <div class="map-tooltip pirate-spawn-key" role="status" use:positionMapGuide>
              <strong
                >{$session.threatGuide === 'barbarians'
                  ? 'Barbarian spawn and movement'
                  : 'Pirate spawn positions'}</strong
              >
              <label
                >Incident player
                <select
                  aria-label={$session.threatGuide === 'barbarians'
                    ? 'Barbarian guide incident player'
                    : 'Pirate guide incident player'}
                  value={$session.pirateSpawnPlayer ?? $session.seat ?? $session.view?.activePlayer}
                  onchange={(event) => controller.setPirateSpawnPlayer(Number(event.currentTarget.value))}
                >
                  {#each $session.view?.players ?? [] as player}<option value={player.index}
                      >{player.civilization}</option
                    >{/each}
                </select>
              </label>
              {#if $session.threatGuide === 'barbarians'}
                <div><span class="barbarian-site-key"><CitySiteIcon /></span>New barbarian city</div>
                <div><i class="barbarian-move" aria-hidden="true"></i>Army movement →</div>
                <div><i class="barbarian-reinforce" aria-hidden="true"></i>Reinforcement city</div>
                <p>
                  Barbarians only move toward the cities of the player who triggered the event. If no army can
                  move, a movement event tries to spawn a city instead.
                </p>
                <small>Current legal choices; earlier moves can change later ones.</small>
              {:else}
                <div><i class="pirate-first" aria-hidden="true"></i>First ship</div>
                <div>
                  <i class="pirate-second" aria-hidden="true"></i>Second ship can also use these tiles
                </div>
                <p>
                  The first ship must be adjacent to this civilization’s city if possible. Otherwise, any
                  eligible sea tile. Player units block placement; existing pirates do not.
                </p>
                {#if !$session.view?.pirateSpawns?.find((p) => p.player === ($session.pirateSpawnPlayer ?? $session.seat ?? $session.view?.activePlayer))?.second.length}<p
                  >
                    No eligible sea tiles right now.
                  </p>{/if}
              {/if}
              <small>Shows current positions. They may change before the event.</small>
            </div>
          {/if}
        </div>
        <span></span>
        <button
          class="map-secondary-control"
          title="Zoom in"
          aria-label="Zoom in"
          onclick={() => world?.zoom(0.84)}><Plus size={18} /></button
        ><button
          class="map-secondary-control"
          title="Zoom out"
          aria-label="Zoom out"
          onclick={() => world?.zoom(1.18)}><Minus size={18} /></button
        ><span></span><button
          class="map-view-toggle"
          title={$session.strategyMap ? 'Show 3D map' : 'Show Strategy map'}
          aria-label="Strategy map"
          aria-pressed={$session.strategyMap}
          onclick={() => controller.toggleMapView()}><Layers size={17} /><span>Strategy</span></button
        >
        {#if $session.strategyMap}<details class="strategy-key" use:positionStrategyKey>
            <summary aria-label="Strategy map key" title="Strategy map key"><Info size={18} /></summary>
            <div class="strategy-key-content">
              <p>
                <Swords size={14} />Army (including leaders) · <Ship size={14} />Ships · <Footprints
                  size={14}
                />Settlers
              </p>
              <p>Colors show city and unit owners. Army aboard ships is shown separately on the map.</p>
              <p>
                <Mountain size={14} />Mountains: stop after entry.<br /><Trees size={14} />Forest: no later
                attack that turn.
              </p>
              <p>
                Roads and some abilities bypass these restrictions. Terrain adds no combat value; tap a hex
                for details.
              </p>
            </div>
          </details>{/if}
        {#if fullscreenEnabled}<button
            title={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
            aria-label={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
            onclick={toggleFullscreen}
            >{#if fullscreen}<Minimize size={17} />{:else}<Maximize size={17} />{/if}</button
          >{/if}
      </div>

      {#if $session.mode === 'collect'}<div class="map-instruction">
          <Wheat size={17} /><span
            >Choose up to <strong
              >{(city?.capacity ?? 0) + Number(!!$session.ballcourts && !!city?.ballcourts)}</strong
            > highlighted tiles</span
          ><span class="instruction-count"
            >{$session.selection.reduce((sum, choice) => sum + choice.times, 0)} / {(city?.capacity ?? 0) +
              Number(!!$session.ballcourts && !!city?.ballcourts)}</span
          >
        </div>{/if}
      <div class="city-dock">
        <div class="dock-intro"><Landmark size={19} /><span class="tiny-label">YOUR CITIES</span></div>
        {#each $session.view?.cities ?? [] as c}<button
            class:selected={c.position === $session.city}
            onclick={() => controller.inspectTile(c.position)}
            ><span class="city-thumb"><Landmark size={25} /></span><span
              ><strong
                >{translate(identity?.civilization ?? '')}
                {translate(c.capital ? 'Capital' : 'City')}{#if c.capital}<Crown
                    size={11}
                    aria-label="Original capital"
                  />{/if}</strong
              ><small>{c.position}<CityFacts size={c.size} mood={c.mood} /></small></span
            ><span
              class="city-activation-badge"
              class:used={c.activations > 0}
              class:blocked={!c.canActivate}
              title={`${c.activations} ${c.activations === 1 ? 'activation' : 'activations'} this turn${!c.canActivate ? ' · Cannot activate again' : c.activations ? ` · Next activation: ${c.activationMood}` : ' · First activation keeps mood'}`}
              aria-label={`${c.activations} ${c.activations === 1 ? 'activation' : 'activations'} this turn`}
              ><RotateCw size={12} aria-hidden="true" />{c.activations}</span
            ><ChevronRight size={16} /></button
          >{/each}
      </div>
    </section>
    <div class="board-toolbar" class:has-redo={$session.view?.canRedo} aria-label="Game controls">
      <div class="turn-banner" class:waiting={!!waiting} role="status" aria-live="polite">
        {#if waiting}<Hourglass size={15} aria-hidden="true" />{:else}<span class="turn-light"></span>{/if}
        <div class="turn-copy">
          <strong>{actionTitle}</strong>
          {#if waiting}<span class="waiting-action"
              >{waiting.action}{waiting.source ? ` · ${waiting.source}` : ''}</span
            >{/if}
        </div>
        {#if !waiting && !objectiveDecision && !$session.view?.decision?.endOfAge && ($session.playback?.frame?.round ?? $session.game?.round ?? 1) <= 3}<span
            class="action-markers"
            role="img"
            aria-label={`${totalActions} ${totalActions === 1 ? 'action' : 'actions'} remaining`}
            title={`${totalActions} ${totalActions === 1 ? 'action' : 'actions'} remaining`}
          >
            {#each Array.from({ length: Math.max(3, totalActions) }) as _, index}<span
                class="action-marker"
                class:spent={index >= totalActions}
                aria-hidden="true"
              ></span>{/each}
          </span>{/if}
        {#if $session.view?.stopMovement && !$session.playback}
          <button
            class="finish-movement"
            disabled={$session.pending ||
              !!$session.view?.decision ||
              !!$session.view?.choiceDecision ||
              !!$session.view?.explorationDecision ||
              !!$session.view?.objectiveDecision}
            onclick={() => controller.submit($session.view!.stopMovement!)}
            ><Check size={20} /><span>Finish Move</span></button
          >
        {/if}
      </div>
      <nav class="board-actions" aria-label="Actions">
        <button
          class:active={$session.mode === 'collect'}
          title={collectAvailable
            ? 'Collect resources · 1 action'
            : (city?.reason ?? 'No city can collect resources')}
          data-tutorial="collect"
          aria-label="Collect resources"
          disabled={!$session.view?.canPlay || !collectAvailable || $session.pending}
          onclick={() => {
            confirmEnd = false;
            controller.beginCollect();
          }}><Wheat size={21} /><span>Collect</span></button
        >
        <button
          class:active={$session.mode === 'research'}
          class:inspect-only={!researchAvailable}
          title={researchAvailable ? 'Research tree' : 'View research · Browse advances and costs'}
          data-tutorial="research"
          aria-label="Research tree"
          disabled={$session.seat === undefined}
          onclick={openResearch}
          ><GraduationCap size={21} /><span>Research</span>{#if !researchAvailable}<Eye
              class="action-inspect"
              size={10}
              aria-hidden="true"
            />{/if}</button
        >
        <button
          class:active={$session.mode === 'city' && $session.cityTab === 'build'}
          class:inspect-only={!cityActionsAvailable}
          aria-label="Manage cities"
          title={cityActionsAvailable
            ? 'Build, recruit and improve happiness'
            : 'View cities · Browse buildings, units and costs'}
          disabled={!city}
          onclick={() => {
            confirmEnd = false;
            controller.openCities();
          }}
          ><Hammer size={21} /><span>Cities</span>{#if !cityActionsAvailable}<Eye
              class="action-inspect"
              size={10}
              aria-hidden="true"
            />{/if}</button
        >
        <button
          class="desktop-action"
          class:active={$session.mode === 'city' && $session.cityTab === 'recruit'}
          data-tutorial="recruit"
          aria-label="Recruit units"
          title="Recruit units · 1 action"
          disabled={!$session.view?.canPlay || !city || $session.pending}
          onclick={() => {
            confirmEnd = false;
            controller.openCities(undefined, 'recruit');
          }}><Users size={21} /><span>Recruit</span></button
        >
        <button
          class="desktop-action"
          class:active={$session.mode === 'happiness'}
          data-tutorial="happiness"
          aria-label="Increase happiness"
          title="Increase happiness · Select cities on the map"
          disabled={!$session.view?.canPlay || !city || $session.pending}
          onclick={() => {
            confirmEnd = false;
            controller.beginHappiness();
          }}><Smile size={21} /><span>Happiness</span></button
        >
        <button
          class:active={$session.mode === 'settlers' || !!$session.view?.stopMovement}
          class:movement-in-progress={!!$session.view?.stopMovement}
          class:inspect-only={!settlersAvailable}
          data-tutorial="movement"
          aria-label="Move units and found cities"
          title={$session.view?.nomadCities?.length
            ? 'Move units or Nomad cities · Found cities'
            : 'Move armies, settlers and ships · Found cities'}
          disabled={$session.seat === undefined}
          onclick={() => {
            confirmEnd = false;
            controller.openSettlers();
          }}
          ><Footprints size={21} /><span>Move</span>{#if !settlersAvailable}<Eye
              class="action-inspect"
              size={10}
              aria-hidden="true"
            />{/if}</button
        >

        <button
          title={abilitiesAvailable
            ? 'Abilities and cultural influence'
            : 'No abilities or influence targets available'}
          data-tutorial="influence"
          aria-label="Abilities and cultural influence"
          class:active={$session.abilitiesOpen}
          disabled={!abilitiesAvailable || $session.pending}
          onclick={() => {
            controller.closeActivity();
            controller.patch({ mode: 'overview', tilePanel: false, abilitiesOpen: !$session.abilitiesOpen });
          }}><Sparkles size={21} /><span>Abilities</span></button
        >
        <button
          title="Undo last action"
          data-tutorial="undo"
          aria-label="Undo last action"
          disabled={!$session.view?.canUndo || $session.pending}
          onclick={() => controller.submit('Undo')}><Undo2 size={19} /><span>Undo</span></button
        >
        {#if $session.view?.canRedo}<button
            title="Redo last undone action"
            aria-label="Redo last undone action"
            disabled={$session.pending}
            onclick={() => controller.submit('Redo')}><Redo2 size={19} /><span>Redo</span></button
          >{/if}
        <button
          class:active={confirmEnd}
          class:end-turn-ready={readyToEnd}
          title="End turn"
          data-tutorial="end"
          aria-label="End turn"
          disabled={!$session.view?.canEndTurn || $session.pending}
          onclick={() => {
            controller.closeActivity();
            controller.patch({ mode: 'overview', tilePanel: false });
            if (totalActions || tradeWarning) confirmEnd = !confirmEnd;
            else controller.submit({ Playing: 'EndTurn' });
          }}><Flag size={19} /><span>End turn</span></button
        >
      </nav>
      {#if !$session.playback}<ContextualCards {controller} context="after-battle" />{/if}
    </div>
    <nav class="table-tools" aria-label="Table controls">
      {#if lastTurn && !$session.analysis}<button
          class="last-turn-button"
          title="Replay all actions since your last turn"
          aria-label="Replay since your last turn"
          disabled={$session.pending}
          onclick={() => controller.replayLastTurn()}><History size={19} /><span>Since my turn</span></button
        >{/if}
      <button
        class:active={$session.activityOpen && $session.tab === 'journal'}
        title="Journal"
        aria-label="Open journal"
        aria-expanded={$session.activityOpen && $session.tab === 'journal'}
        onclick={() => toggleActivity('journal')}><ScrollText size={19} /><span>Journal</span></button
      >
      <button
        class:active={$session.activityOpen && $session.tab === 'chat'}
        title="Chat"
        aria-label="Open chat"
        aria-expanded={$session.activityOpen && $session.tab === 'chat'}
        onclick={() => toggleActivity('chat')}
        ><MessageCircle size={19} /><span>Chat</span>{#if $session.unread}<span class="unread"
            >{$session.unread}</span
          >{/if}</button
      >
      <span class="tool-divider" aria-hidden="true"></span>
      <button
        class:active={$session.sound}
        title={`Sound ${$session.sound ? 'on' : 'off'} · All BGS games`}
        aria-label="Sound"
        aria-pressed={$session.sound}
        onclick={() => controller.setGlobalPreference('sound', !$session.sound)}
        >{#if $session.sound}<Volume2 size={19} />{:else}<VolumeX size={19} />{/if}</button
      >
      <button
        class:active={$session.colorBlind}
        title={`Color-blind mode ${$session.colorBlind ? 'on' : 'off'} · All BGS games`}
        aria-label="Color-blind mode"
        aria-pressed={$session.colorBlind}
        onclick={() => controller.setGlobalPreference('colorBlind', !$session.colorBlind)}
        ><Shapes size={19} /></button
      >
    </nav>
    {#if choiceDecision || objectiveDecision || ($session.game && !$session.view?.supportedPhase && $session.seat === $session.view?.activePlayer) || $session.mode === 'collect' || confirmEnd || ($session.error && $session.mode === 'overview')}
      <section
        class="action-panel floating-panel"
        class:board-collection={$session.mode === 'collect'}
        aria-label="Current action"
        data-tutorial={choiceDecision || objectiveDecision
          ? 'decision'
          : $session.mode === 'collect'
            ? 'collect'
            : undefined}
      >
        {#if !choiceDecision && !objectiveDecision && $session.view?.supportedPhase}<button
            class="icon-button close-action"
            aria-label="Close action"
            onclick={closeAction}><X size={18} /></button
          >{/if}
        {#if choiceDecision}
          {#if $session.view?.influenceContext}<InfluenceFlow
              context={$session.view.influenceContext}
            />{:else}<h2>{choiceDecision.name}</h2>{/if}
          {#if choiceDecision.preview}
            <section class="decision-card-preview" aria-label="Discarded card rules">
              <h3>{choiceDecision.preview.name}</h3>
              {#if choiceDecision.preview.affected}<small>Affects: {choiceDecision.preview.affected}</small
                >{/if}
              {#each choiceDecision.preview.rules as rule}<p><ResourceText text={rule} /></p>{/each}
            </section>
          {/if}
          <div class="collection-choices" class:binary-choices={choiceDecision.binary}>
            {#each choiceDecision.choices as choice, index}<button
                class="secondary wide"
                aria-label={choice.pile ? pileText(choice.pile) : choice.name}
                aria-describedby={choice.description ? `choice-consequence-${index}` : undefined}
                disabled={$session.pending}
                onclick={() => controller.submit(choice.action)}
                >{#if choice.pile}<ResourceAmount pile={choice.pile} />{:else}<span class="choice-outcome"
                    ><strong>{choice.name}</strong>{#if choice.description}<small
                        id={`choice-consequence-${index}`}>{choice.description}</small
                      >{/if}</span
                  >{/if}<ArrowRight size={17} /></button
              >{/each}
          </div>
        {:else if objectiveDecision}
          <h2>{objectiveDecision.name}</h2>
          <p><ResourceText text={objectiveDecision.description} /></p>
          {#each objectiveDecision.cards as card (card.id)}
            <div class="objective-claim">
              <small>Card: {card.name.replaceAll('/', ' / ')}</small>
              <button
                class="primary wide"
                disabled={$session.pending}
                onclick={() => controller.submit(card.action)}
              >
                Claim {formatPoints(objectiveDecision.points)} points <Check size={17} />
              </button>
            </div>
          {/each}
          <p class="claim-note">Claiming discards the card, including its other objective.</p>
          {#if objectiveDecision.skip}
            <button
              class="secondary wide"
              disabled={$session.pending}
              onclick={() => objectiveDecision?.skip && controller.submit(objectiveDecision.skip)}
              >Keep card</button
            >
          {/if}
        {:else if !$session.view?.supportedPhase && $session.game}
          <h2>Decision not available yet</h2>
          <p>The controls for this phase are still being built.</p>
        {:else if $session.mode === 'collect'}
          <header class="collection-heading">
            <h2>
              Collect{#if ($session.view?.cities.length ?? 0) <= 1}<span class="movement-origin"
                  >{$session.city}</span
                >{/if}
            </h2>
            {#if ($session.view?.cities.length ?? 0) > 1}
              <nav class="collection-city-picker" aria-label="Collect from city">
                {#each $session.view?.cities ?? [] as c}
                  <button
                    class:active={c.position === $session.city}
                    aria-pressed={c.position === $session.city}
                    aria-label={`Collect from ${c.position}, ${c.mood}, capacity ${c.capacity}`}
                    title={`${c.position} · ${c.mood} · Capacity ${c.capacity}${c.activations ? ` · Activated ${c.activations} times` : ''}`}
                    disabled={$session.pending}
                    onmouseenter={() => world?.highlightCoordinate(c.position)}
                    onmouseleave={() => world?.highlightCoordinate(null)}
                    onfocus={() => world?.highlightCoordinate(c.position)}
                    onblur={() => world?.highlightCoordinate(null)}
                    onclick={() => controller.switchCollectionCity(c.position)}
                    >{c.position}<CityFacts size={c.size} mood={c.mood} />{#if c.activations}<span
                        class="city-activation-count"><RotateCw size={12} />{c.activations}</span
                      >{/if}</button
                  >
                {/each}
              </nav>
            {/if}
          </header>
          {#if ($session.view?.collectActions?.length ?? 0) > 1}<div class="variant-picker">
              {#each $session.view?.collectActions ?? [] as variant}<button
                  class:selected={JSON.stringify($session.collectVariant) === JSON.stringify(variant.value)}
                  aria-pressed={JSON.stringify($session.collectVariant) === JSON.stringify(variant.value)}
                  disabled={$session.pending}
                  onclick={() => controller.switchCollectVariant(variant.value)}
                  >{variant.name}{#if Object.values(variant.payment ?? {}).some(Boolean)}<ResourceAmount
                      pile={variant.payment!}
                    />{/if}{#if !variant.free}<Zap size={12} />1{/if}</button
                >{/each}
            </div>{/if}
          <ContextualCards {controller} context="collect" />
          {#if city && !city.reason}<CollectionBonusHints {city} selection={$session.selection} />{/if}
          {#if city?.ballcourts}<label class="ballcourts-toggle"
              ><input
                type="checkbox"
                checked={$session.ballcourts ?? false}
                onchange={(e) => controller.setBallcourts(e.currentTarget.checked)}
              />
              Ballcourts · +1 tile <ResourceAmount pile={{ mood_tokens: 1 }} /></label
            >{/if}
          {#if city && city.activations > 0}
            {@const Before = city.mood === 'Happy' ? Smile : city.mood === 'Angry' ? Frown : Meh}
            {@const After =
              city.activationMood === 'Happy' ? Smile : city.activationMood === 'Angry' ? Frown : Meh}
            <details class="collection-activation">
              <summary>
                <TriangleAlert size={16} /><span
                  >{city.canActivate ? 'Activate again' : 'City exhausted'}</span
                >
                {#if city.canActivate && city.mood !== city.activationMood}<span
                    class="collection-mood-change"
                    aria-label={`${city.mood} becomes ${city.activationMood}`}
                    ><span class="city-mood" data-mood={city.mood.toLowerCase()}><Before size={16} /></span
                    ><ArrowRight size={13} /><span
                      class="city-mood"
                      data-mood={city.activationMood.toLowerCase()}><After size={16} /></span
                    ></span
                  >{/if}
                <ChevronRight size={14} />
              </summary>
              <ActivationStatus {city} warning />
            </details>
          {/if}
          {#if $session.collectionTile}<div
              class="collection-tile-options"
              role="group"
              aria-label={`Resource at ${$session.collectionTile}`}
            >
              <strong>{$session.collectionTile}</strong>
              {#each city?.choices.filter((c) => c.position === $session.collectionTile) ?? [] as choice}{@const waste =
                  collectionStorageWaste(
                    choice,
                    $session.selection,
                    city!,
                    current?.resources,
                    current?.resource_limit,
                    Number(!!$session.ballcourts && !!city?.ballcourts),
                  )}<button
                  class:storage-overflow={Object.values(waste).some(Boolean)}
                  class="secondary"
                  onclick={() => {
                    controller.toggleChoice(choice);
                    controller.patch({ collectionTile: null });
                  }}
                  ><ResourceAmount pile={choice.pile} /><CollectionIndicators
                    {waste}
                    bonuses={collectionBonusIndicators(
                      choice,
                      $session.selection,
                      city!,
                      Number(!!$session.ballcourts && !!city?.ballcourts),
                    )}
                  /></button
                >{/each}
            </div>{/if}
          <details class="collection-tile-list" open={!!boardError}>
            <summary
              title={`Choose from this city's tile or adjacent tiles.${city?.maxRange2 ? ` Husbandry allows up to ${city.maxRange2} land tiles two spaces away.` : ''}`}
            >
              <span
                >Tiles <b
                  >{$session.selection.reduce((sum, c) => sum + c.times, 0)} / {(city?.capacity ?? 0) +
                    Number(!!$session.ballcourts && !!city?.ballcourts)}</b
                ></span
              >
              {#if city}<CollectionCapacity size={city.size} mood={city.mood} />{/if}
              <span class="collection-list-label">List <ChevronRight size={14} /></span>
            </summary>
            <div class="collection-choices">
              {#each city?.choices ?? [] as choice}{@const resource = Object.keys(
                  choice.pile,
                )[0] as Resource}{@const Icon = icons[resource] ?? Wheat}{@const selected =
                  $session.selection.some(
                    (c) =>
                      c.position === choice.position &&
                      JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                  )}{@const waste = collectionStorageWaste(
                  choice,
                  $session.selection,
                  city!,
                  current?.resources,
                  current?.resource_limit,
                  Number(!!$session.ballcourts && !!city?.ballcourts),
                )}<button
                  class:selected
                  class:storage-overflow={Object.values(waste).some(Boolean)}
                  aria-label={`${choice.position}: ${pileText(collectionYield(choice, $session.selection))}${choice.bonuses?.length ? ` · ${collectionBonusLabel(choice)}` : ''}${Object.values(waste).some(Boolean) ? ` · ${pileText(waste)} will be lost to the storage limit` : ''}`}
                  onmouseenter={() => world?.highlightCoordinate(choice.position)}
                  onmouseleave={() => world?.highlightCoordinate(null)}
                  onfocus={() => world?.highlightCoordinate(choice.position)}
                  onblur={() => world?.highlightCoordinate(null)}
                  onclick={() => controller.toggleChoice(choice)}
                  disabled={$session.pending}
                  aria-pressed={selected}
                  ><span class="choice-icon {resource}"><Icon size={21} /></span><span
                    ><strong
                      >{pileText(collectionYield(choice, $session.selection))}<CollectionIndicators
                        {waste}
                        bonuses={collectionBonusIndicators(
                          choice,
                          $session.selection,
                          city!,
                          Number(!!$session.ballcourts && !!city?.ballcourts),
                        )}
                      /></strong
                    ><small>Tile {choice.position}</small>{#if choice.bonuses?.length}<small
                        class="collection-bonus"
                        title={collectionBonusLabel(choice)}>{collectionBonusLabel(choice)}</small
                      >{/if}</span
                  ><span class="choice-check"
                    >{#if selected && city && city.maxPerTile > 1}{$session.selection.find(
                        (c) =>
                          c.position === choice.position &&
                          JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                      )?.times}{:else if selected}<Check size={14} />{:else}<Plus size={13} />{/if}</span
                  ></button
                >{/each}
            </div>
          </details>
          <div class="collection-confirm">
            {#if $session.preview}<div class="collection-summary">
                {#if Object.values($session.preview.waste).some((n) => n)}<div
                    class="collection-storage-warning warning"
                    id="collection-storage-warning"
                    role="status"
                  >
                    <TriangleAlert size={16} aria-hidden="true" /><span
                      >Storage limit: <ResourceAmount pile={$session.preview.waste} compact={false} /> will be lost.</span
                    >
                  </div>{/if}
                {#each $session.preview.effects ?? [] as effect}
                  {#if /^(Added|Gain|Convert) \d/.test(effect.description)}
                    <div class="collection-included">
                      <span>{effect.source}</span><ResourceText text={effect.description} /><small
                        >included</small
                      >
                    </div>
                  {/if}
                {/each}
                {#if city && city.activationMood !== city.mood}<span class="activation-inline"
                    ><strong>{city.mood} → {city.activationMood}</strong> after activation</span
                  >{/if}
              </div>{/if}
            <button
              class="primary wide collect-submit"
              disabled={!$session.preview || $session.pending}
              title={`${collectionCostLabel} · Activates this city`}
              aria-label={`Collect resources${$session.preview ? `: ${pileText($session.preview.total)}` : ''} · ${collectionCostLabel}`}
              aria-describedby={Object.values($session.preview?.waste ?? {}).some(Boolean)
                ? 'collection-storage-warning'
                : undefined}
              onclick={() => controller.collect()}
            >
              <span>{$session.pending ? 'Confirming…' : 'Collect'}</span>
              {#if $session.preview}<ResourceAmount pile={$session.preview.total} />{/if}
              <span class="collection-action-cost"
                >{#if Object.values(collectionCost).some(Boolean)}Pay <ResourceAmount
                    pile={collectionCost}
                  /><span>·</span>{/if}<Zap size={14} />{collectionFree ? 0 : 1}</span
              >
              <ArrowRight size={17} />
            </button>
          </div>
        {:else if confirmEnd}
          <h2>End turn?</h2>
          {#if totalActions}<p>
              You have {totalActions} unused {totalActions === 1 ? 'action' : 'actions'}.
            </p>{/if}
          {#if tradeWarning}
            <div class="end-trade-warning" role="status">
              <strong>Trade Routes next turn</strong>
              <p><ResourceAmount pile={tradeWarning.waste} /> may be wasted at your current storage.</p>
              <small>Based on current routes. Other players’ moves may change this.</small>
            </div>
          {/if}
          <div class="end-confirm">
            <button class="secondary" onclick={() => (confirmEnd = false)}>Keep playing</button><button
              class="primary"
              disabled={$session.pending || !$session.view?.canEndTurn}
              onclick={() => {
                confirmEnd = false;
                controller.submit({ Playing: 'EndTurn' });
              }}>End turn <ArrowRight size={16} /></button
            >
          </div>
        {/if}
        {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
      </section>
    {/if}
    {#if $session.mode === 'happiness' && !$session.view?.explorationDecision && !$session.view?.decision && !choiceDecision && !objectiveDecision}<HappinessPanel
        {controller}
        onHighlight={(position) => world?.highlightCoordinate(position)}
      />{/if}
    {#if $session.mode === 'settlers' && !$session.view?.explorationDecision && !$session.view?.decision && !choiceDecision && !objectiveDecision}<SettlerPanel
        {controller}
        onHighlight={(position) => world?.highlightCoordinate(position)}
      />{/if}
    {#if $session.view?.explorationDecision}<ExplorationPanel
        {controller}
        onLocate={() => world?.locateExploration()}
      />{/if}
    {#if researchChoice && $session.mode !== 'research'}<section
        class="action-panel floating-panel decision-panel"
        data-tutorial="decision"
        aria-label="Choose an advance"
      >
        <h2><GraduationCap size={21} />{$session.view?.decision?.name ?? 'Choose an advance'}</h2>
        <button class="primary wide" onclick={openResearch}>Choose an advance<ArrowRight size={16} /></button>
      </section>
    {:else if $session.view?.decision && !researchChoice && !$session.automaticPayment}{#key `${$session.seat}:${$session.game?.log_index}:${JSON.stringify($session.view.decision)}`}<DecisionPanel
          {controller}
          decision={$session.view.decision}
          onHighlight={(position) => world?.highlightCoordinate(position)}
        />{/key}{/if}
    {#if $session.abilitiesOpen && $session.mode === 'overview' && !$session.view?.decision && !choiceDecision && !objectiveDecision}<AbilitiesPanel
        {controller}
        onHighlight={(position) => world?.highlightCoordinate(position)}
      />{/if}
    {#if !$session.playback && ($session.view?.civilizations?.length || $session.view?.civilizationDraft)}<CivilizationPicker
        {controller}
      />{/if}
    <section
      class="activity floating-panel"
      hidden={!$session.activityOpen}
      aria-label="Table activity panel"
    >
      <div class="activity-tabs" role="tablist" aria-label="Table activity">
        <button
          role="tab"
          aria-selected={$session.tab === 'journal'}
          class:active={$session.tab === 'journal'}
          onclick={() => controller.setTab('journal')}><ScrollText size={16} /> Journal</button
        ><button
          role="tab"
          aria-selected={$session.tab === 'chat'}
          class:active={$session.tab === 'chat'}
          onclick={() => controller.setTab('chat')}
          ><MessageCircle size={16} /> Chat {#if $session.unread}<span class="unread">{$session.unread}</span
            >{/if}</button
        ><button
          class="icon-button close-activity"
          aria-label="Close table activity"
          onclick={() => controller.closeActivity()}><X size={17} /></button
        >
      </div>
      <div class="journal" hidden={$session.tab !== 'journal'} role="tabpanel" aria-label="Journal">
        {#each log as entry, index (entry.id)}
          {#if index === 0 || entry.age !== log[index - 1].age || entry.round !== log[index - 1].round}
            <h3 class="journal-round">
              {entry.age === 0 ? 'Setup' : `Age ${entry.age} · Round ${entry.round}`}
            </h3>
          {/if}
          {@render journalRow(entry)}
        {/each}{#if log.length === 0}<p>No actions yet.</p>{/if}
      </div>
      <div
        class="chat-panel"
        hidden={$session.tab !== 'chat'}
        role="tabpanel"
        aria-label="Chat"
        use:chatPanel
      ></div>
    </section>
    {#if !$session.playback && $session.battles?.length}<div class="live-battle floating-panel">
        <BattlePlayback {controller} />
      </div>{/if}
  </main>
  {#if cardReference}<CardReferenceDialog
      card={cardReference}
      onDismiss={() => (cardReference = null)}
    />{/if}
  {#if researchReference}<ResearchTree
      {controller}
      reference={researchReference}
      onDismiss={() => (researchReference = null)}
    />
  {:else if !$session.playback && $session.mode === 'research'}<ResearchTree {controller} />{/if}
  {#if !$session.playback && $session.mode === 'city'}<CityPanel {controller} />{/if}
  {#if $session.scorePlayer !== null}<ScoreDialog
      {controller}
      onLocate={(position) => world?.locateCoordinate(position)}
    />{/if}
  {#if $session.wondersOpen && $session.seat !== undefined}<WondersDialog {controller} />{/if}
  {#if $session.cardsOpen && $session.seat !== undefined}<ActionCardsDialog {controller} />{/if}
  {#if !$session.playback}<CardReveal {controller} />{/if}
  <PlaybackPanel {controller} />
  <PublicEffects {controller} />
  {#if $session.toast}<div class="toast" role="status"><Check size={16} />{$session.toast}</div>{/if}
  {#if $session.objectivesOpen && $session.seat !== undefined}
    <dialog
      class="field-guide objectives-dialog"
      aria-labelledby="objectives-title"
      use:showDialog
      onclose={closeObjectives}
      onclick={(e) => {
        if (e.target === e.currentTarget) closeObjectives();
      }}
      onkeydown={(e) => {
        if (e.key === 'Escape') closeObjectives();
      }}
    >
      <button class="close-guide icon-button" aria-label="Close objectives" onclick={closeObjectives}
        ><X size={20} /></button
      >
      <h2 id="objectives-title">Objectives</h2>
      <p class="guide-intro">{identity?.civilization} · Private hand</p>
      {#each $session.view?.objectiveCards ?? [] as card (card.id)}
        <article
          class="objective-card"
          aria-label={`Objective card: ${card.objectives.map((o) => o.name).join(' / ')}`}
        >
          {#each card.objectives as objective, index}
            {#if index > 0}<div class="objective-divider">or</div>{/if}
            <ObjectiveCondition {objective} />
          {/each}
        </article>
      {:else}
        <p class="objectives-empty">No objective cards in your hand.</p>
      {/each}
      {#if $session.view?.objectiveCards?.length}
        <p class="objectives-note">Complete one objective per card for 2 points.</p>
      {/if}
    </dialog>
  {/if}
  {#if $session.help}<HowToPlay game={$session.game} onClose={closeHelp} />{/if}
</div>
