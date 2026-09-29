<script lang="ts">
  import EventMarkers from './EventMarkers.svelte';
  import TileUnits from './TileUnits.svelte';
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
  import DecisionPanel from './DecisionPanel.svelte';
  import ActionCardsDialog from './ActionCardsDialog.svelte';
  import AbilitiesPanel from './AbilitiesPanel.svelte';
  import type { Controller } from './controller';
  import type { Resource } from './types';
  import { resources, resourceNames, playerColor, playerSymbol } from './types';
  import { journal, pileText } from './model';
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
  let log = $derived($session.game ? journal($session.game).reverse() : []);
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
  let focusTerrain = $derived($session.game?.map.tiles.find(([p]) => p === $session.focus)?.[1]);
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
                  ? 'Moving units'
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
        (p) => {
          if ($session.activityOpen || $session.abilitiesOpen || confirmEnd) dismissMapDetails();
          else controller.selectTile(p);
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
    controller.patch({ mode: 'overview', selection: [], preview: null, error: '' });
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
    controller.patch({ mode: 'research', selectedAdvance: null, error: '', abilitiesOpen: false });
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

<div
  class:dark={$session.dark}
  class:colorblind={$session.colorBlind}
  class:spectating={$session.seat === undefined}
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
            aria-label={`${player.civilization}: ${player.score} victory points. View advances and scores`}
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
      {#if !$session.seaRoutes && !$session.view?.decision && !choiceDecision && !objectiveDecision && $session.mode === 'overview' && $session.focus && $session.focus !== $session.city && typeof focusTerrain === 'string'}<div
          class="tile-inspector"
        >
          <button
            aria-label="Close terrain details"
            onclick={() => controller.patch({ focus: $session.city })}><X size={18} /></button
          ><span class="tiny-label">TILE {$session.focus}</span><strong
            >{focusTerrain === 'Unexplored'
              ? 'Unexplored'
              : focusTerrain === 'Fertile'
                ? 'Fertile land'
                : focusTerrain}</strong
          >
          <p>
            {focusTerrain === 'Unexplored'
              ? 'Move a unit here to reveal the terrain.'
              : focusTerrain === 'Forest'
                ? 'Collect wood here from a nearby city.'
                : focusTerrain === 'Mountain'
                  ? 'Collect ore here from a nearby city.'
                  : focusTerrain === 'Fertile'
                    ? 'Collect food here from a nearby city.'
                    : focusTerrain === 'Water'
                      ? 'Coastal waters. Fishing unlocks food collection.'
                      : 'Barren terrain. Some advances unlock new ways to use it.'}
          </p>
          <TileUnits players={$session.game?.players ?? []} position={$session.focus!} />
        </div>{/if}
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
            controller.patch({ mode: 'overview', abilitiesOpen: !$session.abilitiesOpen });
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
            controller.patch({ mode: 'overview' });
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
      <section class="action-panel floating-panel" aria-label="Current action">
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
          <h2>Collect resources</h2>
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
          <p>
            Choose up to <b>{city?.capacity} {city?.capacity === 1 ? 'tile' : 'tiles'} total</b>: your city’s
            tile or directly adjacent tiles.
            {#if city?.maxRange2}
              With Husbandry, up to {city.maxRange2} selected land
              {city.maxRange2 === 1 ? 'tile' : 'tiles'} may be 2 tiles away.
            {/if}
          </p>
          {#if city}<ActivationStatus {city} warning />{/if}
          <div class="collection-choices">
            {#each city?.choices ?? [] as choice}{@const resource = Object.keys(
                choice.pile,
              )[0] as Resource}{@const Icon = icons[resource] ?? Wheat}{@const selected =
                $session.selection.some(
                  (c) =>
                    c.position === choice.position && JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                )}<button
                class:selected
                onmouseenter={() => world?.highlightCoordinate(choice.position)}
                onmouseleave={() => world?.highlightCoordinate(null)}
                onfocus={() => world?.highlightCoordinate(choice.position)}
                onblur={() => world?.highlightCoordinate(null)}
                onclick={() => controller.toggleChoice(choice)}
                disabled={$session.pending}
                aria-pressed={selected}
                ><span class="choice-icon {resource}"><Icon size={21} /></span><span
                  ><strong>{pileText(choice.pile)}</strong><small>Tile {choice.position}</small></span
                ><span class="choice-check"
                  >{#if selected && city && city.maxPerTile > 1}{$session.selection.find(
                      (c) =>
                        c.position === choice.position &&
                        JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                    )?.times}{:else if selected}<Check size={14} />{:else}<Plus size={13} />{/if}</span
                ></button
              >{/each}
          </div>
          {#if $session.preview}<div class="collection-summary">
              <span>You will collect</span><strong>{pileText($session.preview.total)}</strong
              >{#if Object.values($session.preview.waste).some((n) => n)}<small class="warning"
                  >Storage is full: {pileText($session.preview.waste)} will be lost.</small
                >{/if}
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
        {#each log as entry, index}{@const EntryIcon = journalIcons[entry.kind]}
          {#if index === 0 || entry.age !== log[index - 1].age || entry.round !== log[index - 1].round}
            <h3 class="journal-round">
              {entry.age === 0 ? 'Setup' : `Age ${entry.age} · Round ${entry.round}`}
            </h3>
          {/if}
          <article>
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
              <span
                ><EntryIcon size={13} aria-hidden="true" /><ResourceText
                  text={entry.title}
                  positions={mapPositions}
                  {...coordinateInteraction}
                /></span
              >
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
                      ><ResourceText
                        text={token.label}
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
                  positions={mapPositions}
                  {...coordinateInteraction}
                />
              </p>{/if}
          </article>
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
            Each city piece is worth 1 point. Advances add ½ point each. Completed objectives and wonders can
            add more. End-of-age objectives are checked after everyone’s third turn, during the status phase.
          </p>
        </article>
        <article>
          <Hourglass />
          <h3>Actions</h3>
          <p>
            Each age has three rounds, with one turn per player per round. Each turn gives you three actions.
            Collect resources, research, grow cities, recruit, move, improve happiness, or influence another
            culture.
          </p>
        </article>
        <article>
          <Wheat />
          <h3>Resources</h3>
          <p>
            Select your capital, then Collect resources. Fertile land gives food, forests give wood, and
            mountains give ore.
          </p>
        </article>
        <article>
          <Smile />
          <h3>City mood and storage</h3>
          <p>
            A happy city collects from more tiles. Activating a city again in the same turn lowers its mood.
            Storage limits can make excess resources go to waste.
          </p>
        </article>
      </div>
    </dialog>{/if}
</div>
