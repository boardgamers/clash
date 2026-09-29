<script lang="ts">
  import EventMarkers from './EventMarkers.svelte';
  import TilePanel from './TilePanel.svelte';
  import { onMount } from 'svelte';
  import {
    Landmark,
    Compass,
    Wheat,
    Trees,
    Mountain,
    Lightbulb,
    Coins,
    Smile,
    Drama,
    BookOpen,
    ScrollText,
    MessageCircle,
    ArrowRight,
    ArrowLeft,
    Check,
    Plus,
    Minus,
    Maximize,
    Layers,
    Trophy,
    Flag,
    X,
    ChevronRight,
    Hourglass,
    Undo2,
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
    EyeOff,
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
    Hexagon,
  } from 'lucide-svelte';
  import { mountChat } from '@boardgamers/protocol/chat/dom';
  import { World } from './board';
  import ResearchTree from './ResearchTree.svelte';
  import CityPanel from './CityPanel.svelte';
  import SettlerPanel from './SettlerPanel.svelte';
  import ExplorationPanel from './ExplorationPanel.svelte';
  import ScoreDialog from './ScoreDialog.svelte';
  import WondersDialog from './WondersDialog.svelte';
  import CardReveal from './CardReveal.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';
  import ObjectiveCondition from './ObjectiveCondition.svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import CityFacts from './CityFacts.svelte';
  import CollectionCapacity from './CollectionCapacity.svelte';
  import DecisionPanel from './DecisionPanel.svelte';
  import ActionCardsDialog from './ActionCardsDialog.svelte';
  import AbilitiesPanel from './AbilitiesPanel.svelte';
  import type { Controller } from './controller';
  import type { Resource, JournalEntry } from './types';
  import { resources, resourceNames, playerColor, playerSymbol } from './types';
  import { journal, pileText } from './model';
  import { collectionYield, collectionBonusLabel } from './collection-yield';
  import { movementBonus } from './movement-bonus';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let boardHost: HTMLDivElement;
  let world: World;
  let boardError = $state('');
  let confirmEnd = $state(false);
  let seaTooltipDismissed = $state(false);
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
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
  let city = $derived($session.view?.cities.find((c) => c.position === $session.city));
  let identity = $derived($session.view?.players.find((p) => p.index === $session.seat));
  let log = $derived($session.game ? journal($session.game, $session.view ?? undefined).reverse() : []);
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
  let researchAvailable = $derived($session.view?.advances.some((a) => !!a.action));
  let cityActionsAvailable = $derived(
    $session.view?.cityActions.some(
      (c) =>
        c.buildings.some((b) => b.choices.length > 0) ||
        c.recruits.some((r) => !r.reason && r.available > 0) ||
        c.happiness.some((h) => !!h.action),
    ),
  );
  let settlersAvailable = $derived(
    !!$session.view?.stopMovement ||
      (!!$session.view?.canPlay && totalActions > 0 && !!$session.view?.units?.length),
  );
  let abilitiesAvailable = $derived(
    !!$session.view?.specialActions?.length || !!$session.view?.influence?.length,
  );
  let objectiveDecision = $derived($session.view?.objectiveDecision);
  let choiceDecision = $derived($session.view?.choiceDecision);
  let activeMovementBonus = $derived(movementBonus($session.game));
  let actionTitle = $derived(
    $session.seat === undefined
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
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || document.querySelector('dialog[open]')) return;
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
          if ($session.activityOpen || $session.abilitiesOpen || confirmEnd) dismissMapDetails();
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
      off?.();
      world?.destroy();
    };
  });
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
    if ($session.pending) return;
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
  <article class:journal-event={!!entry.event} class:journal-outcome={outcome}>
    <span
      class="journal-symbol"
      aria-hidden="true"
      style={`--player:${playerColor(entry.player ?? 0, $session.colorBlind)}`}
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
            positions={mapPositions}
            {...coordinateInteraction}
          /></span
        >{/if}
    </div>
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
              ><ResourceText text={token.label} positions={mapPositions} {...coordinateInteraction} /></span
            >
          </span>
        {/each}
      </div>{/if}
    {#if entry.notes.length}<p class="journal-notes">
        <ResourceText text={entry.notes.join(', ')} positions={mapPositions} {...coordinateInteraction} />
      </p>{/if}
    {#if entry.collection}
      {#each entry.collection.effects as effect}
        <div class="collection-included">
          <span><GraduationCap size={13} aria-hidden="true" />{effect.source}</span>
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
            {#if faction}<strong>{faction}</strong>{/if}<span>{explanation.text}</span>
          </div>
        </div>
      {/each}
      {#if entry.event.pending}<p class="event-pending"><Hourglass size={14} />{entry.event.pending}</p>{/if}
      {#if entry.event.info}
        <details class="event-rules">
          <summary aria-label={`${entry.title} rules`}
            ><BookOpen size={13} /> Rules <ChevronRight size={13} /></summary
          >
          {#each entry.event.info.rules as rule}
            <p><ResourceText text={rule} positions={mapPositions} {...coordinateInteraction} /></p>
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
  class:dark={$session.dark}
  class:colorblind={$session.colorBlind}
  class:spectating={$session.seat === undefined}
  class:board-interacting={$session.mode === 'collect' || $session.mode === 'settlers' || $session.tilePanel}
  class="game-shell"
>
  <header class="masthead">
    <div class="brand-block">
      <button
        class="brand"
        onclick={() => controller.commands.openBoardgame()}
        aria-label="About Clash of Cultures"
        ><span class="brand-name">Clash <i>of</i> Cultures</span><span class="mobile-era"
          >Age {['I', 'II', 'III', 'IV', 'V', 'VI'][($session.game?.age ?? 1) - 1] ?? 'VI'} · {($session.game
            ?.round ?? 1) > 3
            ? 'End of age'
            : `Round ${$session.game?.round ?? 1}/3`}</span
        ></button
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
    <div class="age-track" role="group" aria-label={`Age ${$session.game?.age ?? 1} of 6`}>
      {#each [1, 2, 3, 4, 5, 6] as age}
        <span
          class="age-step"
          class:current={age === ($session.game?.age ?? 1)}
          class:past={age < ($session.game?.age ?? 1)}
          title={`Age ${age} of 6`}
        >
          {['I', 'II', 'III', 'IV', 'V', 'VI'][age - 1]}
          {#if age === ($session.game?.age ?? 1)}
            <small class="round-label"
              >{($session.game?.round ?? 1) > 3
                ? 'End of age'
                : `Round ${$session.game?.round ?? 1}/3`}</small
            >
          {/if}
        </span>
      {/each}
    </div>
    <nav class="header-actions">
      {#if $session.seat !== undefined}
        <button
          class="text-button cards-button"
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
      <section class="resource-bar" aria-label="Your resources">
        {#each resources as resource}{@const Icon = icons[resource]}
          <div
            class="resource"
            role="img"
            aria-label={`${resourceNames[resource]}: ${current.resources?.[resource] ?? 0}${current.resource_limit?.[resource] !== undefined ? `, storage limit ${current.resource_limit[resource]}` : ''}`}
            title={`${resourceNames[resource]}${current?.resource_limit?.[resource] !== undefined ? ` · Storage limit ${current.resource_limit[resource]}` : ''}`}
          >
            <span class="resource-icon {resource}"><Icon size={18} strokeWidth={1.65} /></span><span
              ><strong
                >{current.resources?.[resource] ?? 0}{#if current.resource_limit?.[resource] !== undefined}<em
                  >
                    / {current.resource_limit[resource]}</em
                  >{/if}</strong
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
      <div class="player-list" aria-label="Civilizations">
        {#each $session.view?.players ?? [] as player}<button
            class="player-card"
            class:active={player.index === $session.view?.activePlayer}
            style={`--player:${playerColor(player.index, $session.colorBlind)}`}
            title={`Inspect ${player.civilization}: advances and victory points`}
            aria-label={`${player.civilization}: ${player.score} victory points. View resources, advances and scores`}
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
            <span class="player-emblem"
              >{#if $session.colorBlind}<span class="ownership-symbol" aria-hidden="true"
                  >{playerSymbol(player.index)}</span
                >{:else}<CivilizationEmblem civilization={player.civilization} size={24} />{/if}</span
            ><span class="player-info"
              ><strong>{player.civilization}</strong><small
                >{player.index === $session.seat ? 'You' : player.name}</small
              ><span class="player-events"><EventMarkers remaining={player.eventTokens} /></span></span
            ><span class="player-score">{player.score}<Trophy size={10} /></span
            >{#if player.index === $session.view?.activePlayer}<span class="active-dot" title="Current player"
              ></span>{/if}
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
        <span></span>
        <button title="Zoom in" aria-label="Zoom in" onclick={() => world?.zoom(0.84)}
          ><Plus size={18} /></button
        ><button title="Zoom out" aria-label="Zoom out" onclick={() => world?.zoom(1.18)}
          ><Minus size={18} /></button
        ><span></span><button
          title={$session.unitBadges ? 'Hide unit badges' : 'Show unit badges'}
          aria-label="Unit badges"
          aria-pressed={$session.unitBadges}
          class:active={$session.unitBadges}
          onclick={() => controller.toggleUnitBadges()}
          >{#if $session.unitBadges}<Eye size={18} />{:else}<EyeOff size={18} />{/if}</button
        ><button
          title={$session.topDown ? 'Switch to 3D view' : 'Switch to 2D overview'}
          aria-label="Toggle top-down view"
          aria-pressed={$session.topDown}
          onclick={() => controller.toggleMapView()}><Layers size={18} /></button
        ><button title="Reset camera" aria-label="Reset camera" onclick={() => world?.reset()}
          ><Maximize size={17} /></button
        >
      </div>
      {#if $session.mode === 'collect'}<div class="map-instruction">
          <Wheat size={17} /><span>Choose up to <strong>{city?.capacity}</strong> highlighted tiles</span
          ><span class="instruction-count">{$session.selection.length} / {city?.capacity}</span>
        </div>{/if}
      <div class="city-dock">
        <div class="dock-intro"><Landmark size={19} /><span class="tiny-label">YOUR CITIES</span></div>
        {#each $session.view?.cities ?? [] as c}<button
            class:selected={c.position === $session.city}
            onclick={() => controller.openCities(c.position)}
            ><span class="city-thumb"><Landmark size={25} /></span><span
              ><strong
                >{identity?.civilization}
                {c.capital ? 'Capital' : 'City'}{#if c.capital}<Crown
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
    <div class="board-toolbar" aria-label="Game controls">
      <div class="turn-banner">
        <span class="turn-light"></span><strong>{actionTitle}</strong
        >{#if !objectiveDecision && !$session.view?.decision?.endOfAge && ($session.game?.round ?? 1) <= 3}<span
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
      </div>
      <nav class="board-actions" aria-label="Actions">
        <button
          class:active={$session.mode === 'collect'}
          title={city?.reason ?? 'Collect resources · 1 action'}
          aria-label="Collect resources"
          disabled={!$session.view?.canPlay || !city || !!city.reason || $session.pending}
          onclick={() => {
            confirmEnd = false;
            controller.beginCollect();
          }}><Wheat size={21} /><span>Collect</span></button
        >
        <button
          class:active={$session.mode === 'research'}
          class:inspect-only={!researchAvailable}
          title={researchAvailable ? 'Research tree' : 'View research · Browse advances and costs'}
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
          class:active={$session.mode === 'city'}
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
          class:active={$session.mode === 'settlers'}
          class:inspect-only={!settlersAvailable}
          aria-label="Move units and found cities"
          title="Move armies, settlers and ships · Found cities"
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
          aria-label="Undo last action"
          disabled={!$session.view?.canUndo || $session.pending}
          onclick={() => controller.submit('Undo')}><Undo2 size={19} /><span>Undo</span></button
        >
        <button
          class:active={confirmEnd}
          class:end-turn-ready={readyToEnd}
          title="End turn"
          aria-label="End turn"
          disabled={!$session.view?.canEndTurn || $session.pending}
          onclick={() => {
            controller.closeActivity();
            controller.patch({ mode: 'overview', tilePanel: false });
            if (totalActions) confirmEnd = !confirmEnd;
            else controller.submit({ Playing: 'EndTurn' });
          }}><Flag size={19} /><span>End turn</span></button
        >
      </nav>
    </div>
    <nav class="table-tools" aria-label="Table controls">
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
      >
        {#if !choiceDecision && !objectiveDecision && $session.view?.supportedPhase}<button
            class="icon-button close-action"
            aria-label="Close action"
            onclick={closeAction}><X size={18} /></button
          >{/if}
        {#if choiceDecision}
          <h2>{choiceDecision.name}</h2>
          <div class="collection-choices">
            {#each choiceDecision.choices as choice}<button
                class="secondary wide"
                disabled={$session.pending}
                onclick={() => controller.submit(choice.action)}
                >{#if choice.pile}<ResourceAmount pile={choice.pile} />{:else}{choice.name}{/if}<ArrowRight
                  size={17}
                /></button
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
                Claim {objectiveDecision.points} points <Check size={17} />
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
          <h2>Collect <span class="movement-origin">{$session.city}</span></h2>
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
          {#if ($session.view?.collectActions?.length ?? 0) > 1}<div class="variant-picker">
              {#each $session.view?.collectActions ?? [] as variant}<button
                  class:selected={JSON.stringify($session.collectVariant) === JSON.stringify(variant.value)}
                  aria-pressed={JSON.stringify($session.collectVariant) === JSON.stringify(variant.value)}
                  onclick={() =>
                    controller.patch({
                      collectVariant: variant.value,
                      selection: [],
                      preview: null,
                      error: '',
                    })}
                  >{variant.name}{#if !variant.free}<Zap size={12} />1{/if}</button
                >{/each}
            </div>{/if}
          <p
            class="collection-limit"
            title={`Choose from this city's tile or adjacent tiles.${city?.maxRange2 ? ` Husbandry allows up to ${city.maxRange2} land tiles two spaces away.` : ''}`}
          >
            Select tiles · {$session.selection.reduce((sum, c) => sum + c.times, 0)} / {city?.capacity}
            {#if city}<CollectionCapacity size={city.size} mood={city.mood} />{/if}
          </p>
          {#if city && city.activations > 0}<ActivationStatus {city} warning />{/if}
          {#if $session.collectionTile}<div
              class="collection-tile-options"
              role="group"
              aria-label={`Resource at ${$session.collectionTile}`}
            >
              <strong>{$session.collectionTile}</strong>
              {#each city?.choices.filter((c) => c.position === $session.collectionTile) ?? [] as choice}<button
                  class="secondary"
                  onclick={() => {
                    controller.toggleChoice(choice);
                    controller.patch({ collectionTile: null });
                  }}><ResourceAmount pile={choice.pile} /></button
                >{/each}
            </div>{/if}
          <details class="collection-tile-list" open={!!boardError}>
            <summary>Tile yields</summary>
            <div class="collection-choices">
              {#each city?.choices ?? [] as choice}{@const resource = Object.keys(
                  choice.pile,
                )[0] as Resource}{@const Icon = icons[resource] ?? Wheat}{@const selected =
                  $session.selection.some(
                    (c) =>
                      c.position === choice.position &&
                      JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                  )}<button
                  class:selected
                  aria-label={`${choice.position}: ${pileText(collectionYield(choice, $session.selection))}${choice.bonuses?.length ? ` · ${collectionBonusLabel(choice)}` : ''}`}
                  onmouseenter={() => world?.highlightCoordinate(choice.position)}
                  onmouseleave={() => world?.highlightCoordinate(null)}
                  onfocus={() => world?.highlightCoordinate(choice.position)}
                  onblur={() => world?.highlightCoordinate(null)}
                  onclick={() => controller.toggleChoice(choice)}
                  disabled={$session.pending}
                  aria-pressed={selected}
                  ><span class="choice-icon {resource}"><Icon size={21} /></span><span
                    ><strong>{pileText(collectionYield(choice, $session.selection))}</strong><small
                      >Tile {choice.position}</small
                    >{#if choice.bonuses?.length}<small
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
                <span>You will collect</span><strong><ResourceAmount pile={$session.preview.total} /></strong
                >{#if Object.values($session.preview.waste).some((n) => n)}<small class="warning"
                    >Storage is full: {pileText($session.preview.waste)} will be lost.</small
                  >{/if}
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
              class="primary wide"
              disabled={!$session.preview || $session.pending}
              onclick={() => controller.collect()}
              >{$session.pending ? 'Confirming…' : 'Collect resources'}<ArrowRight size={17} /></button
            ><span class="action-cost"
              >{($session.view?.collectActions ?? []).find(
                (a) => JSON.stringify(a.value) === JSON.stringify($session.collectVariant),
              )?.free
                ? 'Free action'
                : 'Costs 1 action'} · Activates your city</span
            >
          </div>
        {:else if confirmEnd}
          <h2>End turn?</h2>
          <p>You have {totalActions} unused {totalActions === 1 ? 'action' : 'actions'}.</p>
          <div class="end-confirm">
            <button class="secondary" onclick={() => (confirmEnd = false)}>Keep playing</button><button
              class="primary"
              disabled={$session.pending}
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
    {#if $session.mode === 'settlers' && !$session.view?.explorationDecision && !$session.view?.decision && !choiceDecision && !objectiveDecision}<SettlerPanel
        {controller}
        onHighlight={(position) => world?.highlightCoordinate(position)}
      />{/if}
    {#if $session.view?.explorationDecision}<ExplorationPanel {controller} />{/if}
    {#if $session.view?.decision}{#key `${$session.seat}:${$session.game?.log_index}:${JSON.stringify($session.view.decision)}`}<DecisionPanel
          {controller}
          decision={$session.view.decision}
          onHighlight={(position) => world?.highlightCoordinate(position)}
        />{/key}{/if}
    {#if $session.abilitiesOpen && $session.mode === 'overview' && !$session.view?.decision && !choiceDecision && !objectiveDecision}<AbilitiesPanel
        {controller}
        onHighlight={(position) => world?.highlightCoordinate(position)}
      />{/if}
    {#if $session.view?.civilizations?.length}<section
        class="action-panel floating-panel"
        aria-label="Choose civilization"
      >
        <h2>Choose civilization</h2>
        <div class="decision-options">
          {#each $session.view.civilizations as civilization}<button
              disabled={$session.pending}
              onclick={() => controller.submit(civilization.action)}
              ><CivilizationEmblem civilization={civilization.name} />{civilization.name}</button
            >{/each}
        </div>
      </section>{/if}
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
  </main>
  {#if $session.mode === 'research'}<ResearchTree {controller} />{/if}
  {#if $session.mode === 'city'}<CityPanel {controller} />{/if}
  {#if $session.scorePlayer !== null}<ScoreDialog {controller} />{/if}
  {#if $session.wondersOpen && $session.seat !== undefined}<WondersDialog {controller} />{/if}
  {#if $session.cardsOpen && $session.seat !== undefined}<ActionCardsDialog {controller} />{/if}
  <CardReveal {controller} />
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
  {#if $session.help}<dialog
      class="field-guide"
      use:showDialog
      onclose={closeHelp}
      onclick={(e) => {
        if (e.target === e.currentTarget) closeHelp();
      }}
      onkeydown={(e) => {
        if (e.key === 'Escape') closeHelp();
      }}
    >
      <button class="close-guide icon-button" aria-label="Close field guide" onclick={closeHelp}
        ><X size={20} /></button
      >
      <h2>How to play</h2>
      <p class="guide-intro">Score the most points over six ages.</p>
      <div class="guide-grid">
        <article>
          <Landmark />
          <h3>Scoring</h3>
          <p>
            Each city piece scores 1 point; each advance scores ½ point. Objectives and wonders add more.
            Select a civilization to see its resources, score breakdown and advances.
          </p>
        </article>
        <article>
          <Hourglass />
          <h3>Turns and ages</h3>
          <p>
            Each age has three rounds, with one turn per player per round. You get three actions per turn.
            After round three, resolve end-of-age steps before starting the next age.
          </p>
        </article>
        <article>
          <Wheat />
          <h3>Collecting resources</h3>
          <p>
            Select a city, choose Collect, then select resource icons on the map. Each collection activates
            one city. Choose its own tile or adjacent tiles; some advances extend your reach. Food comes from
            fertile land, wood from forests, and ore from mountains. Excess beyond your storage limit is lost.
          </p>
        </article>
        <article>
          <Smile />
          <h3>City mood</h3>
          <p>
            Happy cities collect resources or recruit units up to their size + 1; neutral cities use their
            size; angry cities are limited to 1. Activating a city again in the same turn lowers its mood.
            Angry cities cannot build new buildings.
          </p>
        </article>
        <article>
          <Smile />
          <h3>Mood tokens and happiness</h3>
          <p>
            <ResourceText
              text="Gaining 1 mood token adds to your supply; it does not change a city's mood. Improving happiness normally costs one action, plus mood tokens equal to the city's size per mood step. You may improve several cities together in that action."
            />
          </p>
        </article>
        <article>
          <GraduationCap />
          <h3>Research</h3>
          <p>
            <ResourceText
              text="Research normally costs one action and 2 resources: any mix of food, ideas and gold. Wood and ore cannot pay for research. Some advances reduce the cost. Civilization advances unlock automatically with their required research, without another payment or action."
            />
          </p>
        </article>
        <article>
          <ScrollText />
          <h3>Events</h3>
          <p>
            Each ordinary advance normally removes one of your three event markers. When the last is removed,
            an event occurs and the markers refill. The countdown carries across turns; civilization advances
            do not remove extra markers. Events can bring benefits, disasters, barbarians or pirates,
            sometimes affecting everyone.
          </p>
        </article>
        <article>
          <Target />
          <h3>Objective cards</h3>
          <p>
            Open Objectives to see your secret goals. Each card offers two alternatives: complete either to
            claim points when prompted. Some can be claimed during play; others are checked at the end of an
            age. Claiming discards the whole card, including its other objective. You may keep it instead.
          </p>
        </article>
        <article>
          <Footprints />
          <h3>Moving on the map</h3>
          <p>
            Select one of your units, then a highlighted destination and confirm the move. You can also select
            a destination first and choose a unit that can reach it. Moving into unexplored land reveals
            terrain. One Move action lets you move up to three groups. A group is one or more units moving
            together from the same tile to the same destination. A new Move action lets units move again;
            mountains and combat can prevent this. Finish moving ends the current action.
          </p>
        </article>
        <article>
          <Ship />
          <h3>Ships and sea movement</h3>
          <p>
            <ResourceText
              text="A Port is a building, not a ship. Open that city's Recruit tab to build a ship, normally for 2 wood and one action. It appears on the sea tile beside the Port. Select the ship, choose Move, then a highlighted sea tile."
            />
          </p>
          <p>
            Starting Move costs one action; sailing normally costs no resources. The same fleet can continue
            through adjacent sea tiles during that move. Exploration or combat ends its movement. Navigation
            adds routes around the map's edge to the next sea space; it is not needed for ordinary sailing.
            The ship button on the map toolbar only shows routes.
          </p>
        </article>
        <article>
          <Hammer />
          <h3>Growing cities</h3>
          <p>
            A building adds one city size and one point. A city's size cannot exceed your total number of
            cities. Recruit settlers and move them to empty land to found more cities; founding costs a
            separate action.
          </p>
        </article>
      </div>
    </dialog>{/if}
</div>
