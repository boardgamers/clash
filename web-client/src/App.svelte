<script lang="ts">
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
  } from 'lucide-svelte';
  import { mountChat } from '@boardgamers/protocol/chat/dom';
  import { World } from './board';
  import ResearchTree from './ResearchTree.svelte';
  import CityPanel from './CityPanel.svelte';
  import SettlerPanel from './SettlerPanel.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import type { Controller } from './controller';
  import type { Resource } from './types';
  import { resources, resourceNames, playerColor, playerSymbol } from './types';
  import { journal, journalParts, pileText } from './model';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let boardHost: HTMLDivElement;
  let world: World;
  let boardError = $state('');
  let confirmEnd = $state(false);
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
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
  let log = $derived($session.game ? journal($session.game) : []);
  let totalActions = $derived($session.game?.actions_left ?? 0);
  let objectiveDecision = $derived($session.view?.objectiveDecision);
  let choiceDecision = $derived($session.view?.choiceDecision);
  let focusTerrain = $derived($session.game?.map.tiles.find(([p]) => p === $session.focus)?.[1]);
  let actionTitle = $derived(
    $session.seat === undefined
      ? 'Spectating'
      : choiceDecision
        ? 'Choose a bonus'
        : objectiveDecision
          ? 'Objective available'
          : $session.view?.stopMovement
            ? 'Moving settlers'
            : $session.view?.canPlay
              ? 'Your turn'
              : `${$session.view?.players.find((p) => p.index === $session.view?.activePlayer)?.civilization ?? 'Opponent'}’s turn`,
  );
  onMount(() => {
    try {
      world = new World(boardHost, (p) => controller.selectTile(p));
      const off = session.subscribe((s) => world.update(s));
      return () => {
        off();
        world.destroy();
      };
    } catch (e) {
      boardError =
        'The 3D map could not start on this device. You can still select your city and collect resources with the controls.';
    }
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
  function openResearch() {
    confirmEnd = false;
    controller.closeActivity();
    controller.patch({ mode: 'research', selectedAdvance: null, error: '' });
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

<div class:dark={$session.dark} class:colorblind={$session.colorBlind} class="game-shell">
  <header class="masthead">
    <button
      class="brand"
      onclick={() => controller.commands.openBoardgame()}
      aria-label="About Clash of Cultures"><span class="brand-name">Clash <i>of</i> Cultures</span></button
    >
    <div class="age-track" aria-label={`Age ${$session.game?.age ?? 1} of 6`}>
      <span>THE AGES</span>{#each [1, 2, 3, 4, 5, 6] as age}<span
          class:current={age === ($session.game?.age ?? 1)}
          class:past={age < ($session.game?.age ?? 1)}>{['I', 'II', 'III', 'IV', 'V', 'VI'][age - 1]}</span
        >{/each}
    </div>
    <nav class="header-actions">
      {#if $session.seat !== undefined}
        <button
          class="text-button objectives-button"
          aria-label={`Objectives: ${$session.view?.objectiveCards?.length ?? 0} ${$session.view?.objectiveCards?.length === 1 ? 'card' : 'cards'}`}
          onclick={() => controller.patch({ objectivesOpen: true })}
          ><Target size={17} /> Objectives
          <span class="card-count">{$session.view?.objectiveCards?.length ?? 0}</span></button
        >
      {/if}
      <button class="text-button" onclick={() => controller.patch({ help: true })}
        ><BookOpen size={17} /> How to play</button
      >
    </nav>
  </header>
  <section class="resource-bar" aria-label="Your resources">
    <div class="civilization-tag">
      <span class="tiny-label">{$session.seat === undefined ? 'THE TABLE' : 'YOUR CIVILIZATION'}</span><strong
        >{identity?.civilization ?? 'Spectator'}</strong
      >
    </div>
    {#each resources as resource}{@const Icon = icons[resource]}
      <div
        class="resource"
        title={`${resourceNames[resource]}${current?.resource_limit?.[resource] !== undefined ? ` · Storage limit ${current.resource_limit[resource]}` : ''}`}
      >
        <span class="resource-icon {resource}"><Icon size={21} strokeWidth={1.65} /></span><span
          ><small>{resourceNames[resource]}</small><strong
            >{current
              ? (current.resources?.[resource] ?? 0)
              : '—'}{#if current?.resource_limit?.[resource] !== undefined}<em>
                / {current.resource_limit[resource]}</em
              >{/if}</strong
          ></span
        >
      </div>{/each}
    <div class="round-label">
      {#if ($session.game?.round ?? 1) > 3}
        <span class="tiny-label">AGE END</span><strong class="status-label">Status</strong>
      {:else}
        <span class="tiny-label">ROUND</span><strong>{$session.game?.round ?? 1} <span>/ 3</span></strong>
      {/if}
    </div>
  </section>
  <main class="play-layout">
    <section class="map-section" aria-label="The civilization map">
      <div class="map-world" bind:this={boardHost}></div>
      <div class="map-vignette"></div>
      <div class="player-list" aria-label="Civilizations">
        {#each $session.view?.players ?? [] as player}<button
            class="player-card"
            class:active={player.index === $session.view?.activePlayer}
            style={`--player:${playerColor(player.index, $session.colorBlind)}`}
            onclick={() => controller.commands.openPlayer(player.index)}
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
                >{:else if $session.avatars[player.index]}<img
                  src={$session.avatars[player.index]}
                  alt=""
                />{:else}<Landmark size={18} />{/if}</span
            ><span class="player-info"
              ><strong>{player.civilization}</strong><small
                >{player.index === $session.seat ? 'You' : player.name}</small
              ></span
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
      {#if $session.focus && $session.focus !== $session.city && typeof focusTerrain === 'string'}<div
          class="tile-inspector"
        >
          <button
            aria-label="Close terrain details"
            onclick={() => controller.patch({ focus: $session.city })}><X size={12} /></button
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
        </div>{/if}
      <div class="map-controls">
        <button title="Zoom in" aria-label="Zoom in" onclick={() => world?.zoom(0.84)}
          ><Plus size={18} /></button
        ><button title="Zoom out" aria-label="Zoom out" onclick={() => world?.zoom(1.18)}
          ><Minus size={18} /></button
        ><span></span><button
          title="Toggle top-down view"
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
                {c.position === $session.view?.cities[0]?.position ? 'Capital' : 'City'}</strong
              ><small>{c.position} <span>·</span> Size {c.size} <span>·</span> {c.mood}</small></span
            ><ChevronRight size={16} /></button
          >{/each}
      </div>
    </section>
    <div class="board-toolbar" aria-label="Game controls">
      <div class="turn-banner">
        <span class="turn-light"></span><strong>{actionTitle}</strong
        >{#if !objectiveDecision && ($session.game?.round ?? 1) <= 3}<span
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
          title="Research tree"
          aria-label="Research tree"
          disabled={$session.seat === undefined}
          onclick={openResearch}><GraduationCap size={21} /><span>Research</span></button
        >
        <button
          class:active={$session.mode === 'city'}
          aria-label="Manage cities"
          title="Build, recruit and improve happiness"
          disabled={!city}
          onclick={() => {
            confirmEnd = false;
            controller.openCities();
          }}><Hammer size={21} /><span>Cities</span></button
        >
        <button
          class:active={$session.mode === 'settlers'}
          aria-label="Move settlers and found cities"
          title="Move settlers and found cities"
          disabled={$session.seat === undefined}
          onclick={() => {
            confirmEnd = false;
            controller.openSettlers();
          }}><Footprints size={21} /><span>Settlers</span></button
        >
        <button
          title="Undo last action"
          aria-label="Undo last action"
          disabled={!$session.view?.canUndo || $session.pending}
          onclick={() => controller.submit('Undo')}><Undo2 size={19} /><span>Undo</span></button
        >
        <button
          class:active={confirmEnd}
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
        ><Eye size={19} /></button
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
          <p>{objectiveDecision.description}</p>
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
          <p>
            Your {city?.mood.toLowerCase()} city can gather from up to <b>{city?.capacity} tiles</b>. Choose
            on the map or below.
          </p>
          <div class="collection-choices">
            {#each city?.choices ?? [] as choice}{@const resource = Object.keys(
                choice.pile,
              )[0] as Resource}{@const Icon = icons[resource] ?? Wheat}{@const selected =
                $session.selection.some(
                  (c) =>
                    c.position === choice.position && JSON.stringify(c.pile) === JSON.stringify(choice.pile),
                )}<button
                class:selected
                onclick={() => controller.toggleChoice(choice)}
                disabled={$session.pending}
                aria-pressed={selected}
                ><span class="choice-icon {resource}"><Icon size={21} /></span><span
                  ><strong>{pileText(choice.pile)}</strong><small>Tile {choice.position}</small></span
                ><span class="choice-check"
                  >{#if selected}<Check size={14} />{:else}<Plus size={13} />{/if}</span
                ></button
              >{/each}
          </div>
          {#if $session.preview}<div class="collection-summary">
              <span>You will collect</span><strong>{pileText($session.preview.total)}</strong
              >{#if Object.values($session.preview.waste).some((n) => n)}<small class="warning"
                  >Storage is full: {pileText($session.preview.waste)} will be lost.</small
                >{/if}{#if $session.preview.moodWillDecrease}<small class="warning"
                  >Activating this city again lowers its mood.</small
                >{/if}
            </div>{/if}
          <button
            class="primary wide"
            disabled={!$session.preview || $session.pending}
            onclick={() => controller.collect()}
            >{$session.pending ? 'Confirming…' : 'Collect resources'}<ArrowRight size={17} /></button
          ><span class="action-cost">Costs 1 action · Activates your city</span>
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
    {#if $session.mode === 'settlers'}<SettlerPanel {controller} />{/if}
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
        {#each [...log].reverse() as entry}{@const EntryIcon = journalIcons[entry.kind]}
          <article>
            <span class="journal-symbol" aria-hidden="true"><EntryIcon size={15} /></span><small
              >{entry.age === 0 ? 'SETUP' : `AGE ${entry.age} · ROUND ${entry.round}`}</small
            >
            {#if entry.setup}
              <p class="journal-setup">
                <strong>{entry.setup.player}</strong><span>· {entry.setup.civilization}</span
                >{#if entry.setup.position}<span class="journal-position" title="Starting location"
                    ><MapPin size={13} aria-hidden="true" /><span class="sr-only">Starts at </span>{entry
                      .setup.position}</span
                  >{/if}
              </p>
            {:else}
              <p>
                {#each journalParts(entry.text) as part}{#if part.resource}{@const ResourceIcon =
                      icons[part.resource]}<span class="journal-resource"
                      ><ResourceIcon size={13} aria-hidden="true" />{part.text}</span
                    >{:else}{part.text}{/if}{/each}
              </p>
            {/if}
          </article>{/each}{#if log.length === 0}<p>No actions yet.</p>{/if}
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
            <section>
              <div class="objective-title">
                <h3>{objective.name}</h3>
                <span>{objective.timing}</span>
              </div>
              <p>{objective.description}</p>
            </section>
          {/each}
        </article>
      {:else}
        <p class="objectives-empty">No objective cards in your hand.</p>
      {/each}
      {#if $session.view?.objectiveCards?.length}
        <p class="objectives-note">
          Complete one objective per card for 2 points. When eligible, choose whether to claim it or keep the
          card. Status phase objectives are checked at the end of an age.
        </p>
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
            add more.
          </p>
        </article>
        <article>
          <Hourglass />
          <h3>Actions</h3>
          <p>
            Each turn gives you three actions. Collect resources, research, grow cities, recruit, move,
            improve happiness, or influence another culture.
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
