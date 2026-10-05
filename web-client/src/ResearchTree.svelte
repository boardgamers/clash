<script lang="ts">
  import { tick, untrack } from 'svelte';
  import {
    X,
    Search,
    Check,
    LockKeyhole,
    ArrowRight,
    Wheat,
    Trees,
    Mountain,
    Lightbulb,
    Coins,
    Smile,
    Drama,
    Link,
    Hammer,
    BookOpen,
    ScrollText,
    Map,
  } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceAmount from './ResourceAmount.svelte';
  import ContextualCards from './ContextualCards.svelte';
  import PaymentPicker from './PaymentPicker.svelte';
  import EventMarkers from './EventMarkers.svelte';
  import CivilizationAdvances from './CivilizationAdvances.svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import ResourceText from './ResourceText.svelte';
  import {
    resourceNames,
    type Resource,
    type Pile,
    type AdvanceView,
    type PublicAdvance,
    type View,
  } from './types';
  import { buildingInfo } from './city';
  import { researchOwners, researchFreeHints, type ResearchReference } from './research-links';
  import { actionReason, pileText } from './model';
  import { groupIcons, researchPresentation } from './research';
  import { researchDecision } from './decision-controls';
  let {
    controller,
    reference,
    onDismiss,
  }: {
    controller: Controller;
    reference?: { target: ResearchReference; view: View };
    onDismiss?: () => void;
  } = $props();
  const session = $derived(controller.session);
  const view = $derived(reference?.view ?? $session.view);
  const choice = $derived(!reference && researchDecision(view) ? view!.decision : null);
  const freeResearch = $derived(!!choice && choice.advanceMode === 'free');
  const borrowing = $derived(choice?.advanceMode === 'borrow');
  let query = $state('');
  let category = $state('All');
  let researchedOnly = $state(false);
  let inspectAvailableOnly = $state(false);
  let inspectedAdvance = $state<string | null>(null);
  let selectedCivilization = $state<string | null>(null);
  const availableOnly = $derived(reference ? inspectAvailableOnly : $session.availableOnly);
  const selectedId = $derived(reference ? inspectedAdvance : $session.selectedAdvance);
  function selectAdvance(id: string | null) {
    selectedCivilization = null;
    if (reference) inspectedAdvance = id;
    else controller.patch({ selectedAdvance: id });
  }
  const statusFilter = $derived(researchedOnly ? 'owned' : availableOnly ? 'available' : 'all');
  function setStatusFilter(value: string) {
    researchedOnly = value === 'owned';
    if (!researchedOnly) {
      if (reference) inspectAvailableOnly = value === 'available';
      else controller.setAvailableOnly(value === 'available');
    }
  }
  let chosenPayment = $state<{ advance: string; payment: Pile } | null>(null);
  const samePayment = (a: Pile, b: Pile) =>
    (Object.keys(resourceNames) as Resource[]).every((r) => (a[r] ?? 0) === (b[r] ?? 0));
  function costLabel(advance: AdvanceView) {
    return (advance.costGroups ?? [{ amount: advance.costAmount, resources: advance.costResources }])
      .map((group) =>
        group.amount === 0
          ? 'No resources'
          : `${group.amount} ${group.resources.length > 1 ? 'total: any mix of ' : ''}${group.resources.map((r) => resourceNames[r]).join(', ')}`,
      )
      .join(' or ');
  }
  const resourceIcons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
    captives: Link,
  };
  let advances = $derived([...(view?.advances ?? [])].sort((a, b) => a.order - b.order));
  const governmentGroups = ['Democracy', 'Autocracy', 'Theocracy'];
  const currentGovernment = $derived(
    advances.find((a) => a.owned && governmentGroups.includes(a.group))?.group,
  );
  function governmentLocked(group: string) {
    return (
      !!currentGovernment &&
      governmentGroups.includes(group) &&
      group !== currentGovernment &&
      !advances.some((a) => a.group === group && a.action)
    );
  }
  function unavailableReason(advance: AdvanceView) {
    return !advance.owned && governmentLocked(advance.group)
      ? `Your government is ${currentGovernment}. Switch to ${advance.group} at the end of an age for 1 mood token + 1 culture token. You must meet its prerequisite; your government advances are replaced with the same number from the new government.`
      : actionReason(advance.reason);
  }
  const availableAdvances = $derived(
    advances.filter((a) => (researchedOnly ? a.owned : !availableOnly || !!a.action)),
  );
  let player = $derived(
    view?.players.find(
      (p) =>
        p.index ===
        (reference ? (reference.target.player ?? $session.seat ?? view?.activePlayer) : $session.seat),
    ),
  );
  let civilizationAdvances = $derived(player?.civilizationAdvances ?? []);
  let visibleCivilizationAdvances = $derived(
    category === 'All' || category === 'Civilization'
      ? civilizationAdvances.filter(
          (a) =>
            (researchedOnly
              ? a.owned
              : !availableOnly ||
                (!a.owned &&
                  a.prerequisites.some((p) => availableAdvances.some((advance) => advance.id === p.id)))) &&
            (matches(a) ||
              a.requirement.toLowerCase().includes(query.trim().toLowerCase()) ||
              a.prerequisites.some((p) => p.name.toLowerCase().includes(query.trim().toLowerCase()))),
        )
      : [],
  );
  let groups = $derived([...new Set(advances.map((a) => a.group))]);
  let selected = $derived(availableAdvances.find((a) => a.id === selectedId));
  let selectedPayment = $derived(
    selected?.payments.find((p) =>
      samePayment(
        p.payment,
        chosenPayment?.advance === selected.id ? chosenPayment.payment : selected.payment,
      ),
    ) ?? selected?.payments[0],
  );
  let visibleGroups = $derived(
    groups.filter(
      (group) =>
        (category === 'All' || group === category) &&
        (availableAdvances.some((a) => a.group === group && matches(a)) ||
          (category === group && governmentLocked(group) && !researchedOnly)),
    ),
  );
  const selectedAction = $derived(choice ? selected?.action : selectedPayment?.action);
  const eventImminent = $derived(
    !reference && (selected ? !!selected.triggersEvent : advances.some((a) => a.triggersEvent)),
  );
  function matches(advance: PublicAdvance) {
    return `${advance.name} ${advance.description} ${researchPresentation(advance).summary}`
      .toLowerCase()
      .includes(query.trim().toLowerCase());
  }
  function close() {
    if (reference) onDismiss?.();
    else if ($session.mode === 'research') controller.patch({ mode: 'overview', selectedAdvance: null });
  }
  async function selectCategory(value: string) {
    category = value;
    selectAdvance(null);
    await tick();
    document.querySelector('.research-tree')?.scrollTo({ top: 0, left: 0 });
  }
  function open(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
  async function showCivilizationAdvance(id: string) {
    setStatusFilter('all');
    query = '';
    if (category !== 'All') category = 'Civilization';
    selectAdvance(null);
    selectedCivilization = id;
    await tick();
    document.getElementById(`civilization-${id}`)?.scrollIntoView({
      block: 'nearest',
      behavior: $session.reducedMotion ? 'instant' : 'smooth',
    });
  }
  async function showAdvance(id: string) {
    if (researchedOnly || !advances.find((a) => a.id === id)?.action) setStatusFilter('all');
    query = '';
    if (category !== 'All') category = advances.find((a) => a.id === id)?.group ?? 'All';
    selectAdvance(id);
    await tick();
    document.getElementById(`research-${id}`)?.scrollIntoView({
      block: 'nearest',
      inline: 'nearest',
      behavior: $session.reducedMotion ? 'instant' : 'smooth',
    });
  }
  $effect(() => {
    const target = reference?.target;
    if (target)
      untrack(() => {
        if (target.civilization) void showCivilizationAdvance(target.id);
        else void showAdvance(target.id);
      });
  });
</script>

<dialog
  class="research-dialog"
  aria-labelledby="research-title"
  use:open
  onclose={close}
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
>
  <header class="research-header" class:event-ready={eventImminent}>
    <div>
      <h2 id="research-title">
        {choice ? (choice.endOfAge ? 'Free advance' : choice.name) : 'Research'}
        {#if reference && player}<span>· {player.civilization}</span>{/if}
        {#if player}<EventMarkers remaining={player.eventTokens} />{/if}
      </h2>
      <p>
        {choice
          ? choice.description
          : 'Each advance scores ½ point. Branches unlock from their first advance.'}
      </p>
      {#if eventImminent}<p
          class="research-event-notice"
          id="research-event-notice"
          title="This advance uses your last event marker. The event resolves after research bonuses and choices."
        >
          <ScrollText size={14} aria-hidden="true" />Next advance triggers an event
        </p>{/if}
    </div>
    <!-- The dialog focuses its first control; avoid opening the mobile keyboard. -->
    <button class="icon-button" aria-label="Close research" onclick={close}><X size={21} /></button>
    <label class="research-search"
      ><Search size={17} /><input
        aria-label="Search research"
        placeholder="Find an effect or advance…"
        bind:value={query}
      /></label
    >
  </header>
  {#if !choice && !reference}<ContextualCards {controller} context="research" />{/if}
  <nav class="research-filters" aria-label="Research categories">
    <select
      class="research-status-filter"
      aria-label="Filter advances"
      value={statusFilter}
      onchange={(event) => setStatusFilter(event.currentTarget.value)}
    >
      <option value="available">Available</option><option value="all">All</option><option value="owned"
        >Researched</option
      >
    </select>
    <button class:active={category === 'All'} onclick={() => selectCategory('All')}>All advances</button>
    {#if player && civilizationAdvances.length}<button
        class:active={category === 'Civilization'}
        onclick={() => selectCategory('Civilization')}
        ><CivilizationEmblem civilization={player.civilization} size={16} />{player.civilization}</button
      >{/if}
    {#each groups as group}{@const Icon = groupIcons[group] ?? BookOpen}<button
        class:active={category === group}
        onclick={() => selectCategory(group)}><Icon size={15} />{group}</button
      >{/each}
  </nav>
  <div class="research-tree" class:civilization-only={category === 'Civilization'} aria-label="Research tree">
    {#if player && visibleCivilizationAdvances.length}
      <CivilizationAdvances
        civilization={player.civilization}
        advances={visibleCivilizationAdvances}
        onPrerequisite={showAdvance}
        selected={selectedCivilization}
      />
    {/if}
    {#each visibleGroups as group}{@const GroupIcon = groupIcons[group] ?? BookOpen}
      {@const lockedGovernment = governmentLocked(group)}
      <section class="research-branch" class:government-locked={lockedGovernment} aria-label={group}>
        <h3>
          <GroupIcon size={18} />{group}
          {#if group === currentGovernment}<span class="government-label"
              ><Check size={12} />Current government</span
            >
          {:else if lockedGovernment}<LockKeyhole size={13} aria-label="Government unavailable" />{/if}
        </h3>
        {#if governmentGroups.includes(group)}<p class="government-rule">
            {#if lockedGovernment}
              <span>Only one government at a time.</span>
              <span class="government-switch"
                >Switch at end of age · <ResourceAmount pile={{ mood_tokens: 1, culture_tokens: 1 }} /></span
              >
            {:else if !currentGovernment}Only one government at a time. You can switch at the end of an age.{/if}
          </p>{/if}
        {#if lockedGovernment && statusFilter === 'available'}<button
            class="show-all-options"
            onclick={() => setStatusFilter('all')}>Show advances</button
          >{/if}
        {#each availableAdvances.filter((a) => a.group === group) as advance, index}{@const presentation =
            researchPresentation(advance)}{@const Icon = presentation.icon}{@const parent = advances.find(
            (a) => a.id === advance.required,
          )}
          {@const warnsEvent = !reference && !!advance.action && !!advance.triggersEvent}
          <article
            id={`research-${advance.id}`}
            class="research-node"
            class:child={statusFilter === 'all' && index > 0}
            class:owned={advance.owned}
            class:available={!!advance.action}
            class:selected={selected?.id === advance.id}
            class:search-muted={!!query.trim() && !matches(advance)}
          >
            <button
              class="research-pick"
              aria-label={`${advance.name}: ${presentation.summary}`}
              aria-pressed={selected?.id === advance.id}
              aria-describedby={warnsEvent ? `research-${advance.id}-event` : undefined}
              title={advance.description}
              onclick={() => selectAdvance(advance.id)}
            >
              <span class="research-node-title"
                ><span class="advance-pictogram"><Icon size={21} /></span><strong>{advance.name}</strong
                >{#if advance.owned}<Check
                    size={16}
                    aria-label="Researched"
                  />{:else if !advance.action && (lockedGovernment || actionReason(advance.reason) || (parent && !parent.owned))}<LockKeyhole
                    size={13}
                    aria-label={lockedGovernment
                      ? 'Switch government at end of age'
                      : !choice && parent && !parent.owned
                        ? `Needs ${parent.name}`
                        : actionReason(advance.reason)}
                  />{/if}</span
              >
              <span class="research-summary"><ResourceText text={presentation.summary} /></span>
            </button>
            <div class="research-effects">
              {#if advance.unlocks}<span
                  title={`${advance.unlocks}: ${buildingInfo[advance.unlocks]?.effect ?? 'Unlocked building'}`}
                  ><Hammer size={12} />{advance.unlocks}</span
                >{/if}{#each Object.entries(advance.bonus ?? {}) as [resource, amount]}{@const BonusIcon =
                  resourceIcons[resource as Resource]}<span
                  title={`Research bonus: ${amount} ${resourceNames[resource as Resource]}`}
                  ><BonusIcon size={12} />+{amount} {resourceNames[resource as Resource]}</span
                >{/each}{#each advance.bonusEffects ?? [] as bonus}<span
                  title={`Gain ${pileText(bonus.pile)} from ${bonus.source}`}
                >
                  +<ResourceAmount pile={bonus.pile} compact /> · {bonus.source}
                </span>{/each}
            </div>
            {#if !advance.owned}<div class="research-node-cost">
              <span
                  class="research-flexible-cost"
                  title={borrowing
                    ? 'Borrow until end of turn'
                    : freeResearch
                      ? 'Free advance'
                      : costLabel(advance)}
                  aria-label={borrowing
                    ? 'Borrow until end of turn'
                    : freeResearch
                      ? 'Free advance'
                      : costLabel(advance)}
                >
                  {#if borrowing}This turn{:else if freeResearch}Free{:else if advance.costAmount === 0}No
                    resources{:else}
                    {#each advance.costGroups ?? [{ amount: advance.costAmount, resources: advance.costResources }] as group, gi}
                      {#if gi > 0}<span>or</span>{/if}<b>{group.amount}</b>
                      {#each group.resources as resource, i}{@const CostIcon = resourceIcons[resource]}
                        {#if i > 0}/{/if}<CostIcon size={13} />
                      {/each}
                      {#if group.resources.length > 1}<span>any mix</span>{/if}
                    {/each}
                  {/if}
                </span><span class="research-availability"
                  >{#if advance.action}
                    {#if warnsEvent}<span
                        id={`research-${advance.id}-event`}
                        class="research-event-warning"
                        title="Uses your last event marker. The event resolves after research bonuses and choices."
                        ><ScrollText size={12} aria-hidden="true" />Triggers an event</span
                      >{:else}Available{/if}
                  {:else if !choice && parent && !parent.owned}
                    <button
                      class="research-advance-link"
                      title={`View ${parent.name}`}
                      onclick={() => showAdvance(parent.id)}>Needs {parent.name}</button
                    >
                  {:else if !lockedGovernment}{actionReason(advance.reason)}{/if}</span
                >
              {#if !borrowing && !freeResearch}
                {#each researchFreeHints(advance, advances) as source}
                  <span class="research-free-hint">
                    <button
                      class="research-advance-link"
                      title="No resource cost; the research action and prerequisites still apply."
                      onclick={() => showAdvance(source.id)}
                      >Free with {source.name}{source.oncePerTurn ? ' · once per turn' : ''}</button
                    >
                  </span>
                {/each}
              {/if}
            </div>{/if}
            {#if researchOwners(view, advance.id, player?.index).length}
              <div class="research-other-owners" aria-label="Other civilizations with this advance">
                {#each researchOwners(view, advance.id, player?.index) as owner}
                  <span title={`Researched by ${owner.civilization} (${owner.name})`}>
                    <CivilizationEmblem civilization={owner.civilization} size={14} />{owner.civilization}
                  </span>
                {/each}
              </div>
            {/if}
            {#each advances.filter((a) => a.required === advance.id && a.group !== advance.group) as unlocked}
              {@const UnlockIcon = researchPresentation(unlocked).icon}
              <button
                class="research-civilization-link"
                onclick={() => showAdvance(unlocked.id)}
                title={`${unlocked.group} · ${unlocked.name}`}
              >
                <UnlockIcon size={14} />Unlocks {unlocked.name}<ArrowRight size={12} />
              </button>
            {/each}
            {#each civilizationAdvances.filter( (a) => a.prerequisites.some((p) => p.id === advance.id) ) as special}
              <button
                class="research-civilization-link"
                onclick={() => showCivilizationAdvance(special.id)}
                title={`${special.active ? 'Unlocked' : 'Unlocks automatically'}: ${special.name}`}
              >
                <CivilizationEmblem civilization={player!.civilization} size={14} />
                {special.active ? 'Unlocked' : 'Unlocks'}
                {special.name}<ArrowRight size={12} />
              </button>
            {/each}
          </article>
        {/each}
      </section>
    {/each}
    {#if !visibleGroups.length && !visibleCivilizationAdvances.length}<p class="research-empty">
        {researchedOnly
          ? 'No researched advances match these filters.'
          : availableOnly
            ? 'No research available with these filters.'
            : query.trim()
              ? `No advances match “${query}”.`
              : 'No advances in this category.'}
        {#if statusFilter !== 'all'}<button class="show-all-options" onclick={() => setStatusFilter('all')}
            >Show all</button
          >{/if}
      </p>{/if}
  </div>
  {#if selected}
    <section class="research-detail" aria-label="Research details">
      <div>
        <h3>
          {selected.name}{#if selected.group === 'Seafaring' && !reference}<button
              class="icon-button sea-map-link"
              aria-label="Show sea routes on map"
              title="Show connected seas and Navigation shortcuts on the map"
              onclick={() => controller.showSeaRoutes()}><Map size={18} /></button
            >{/if}
        </h3>
        <p>
          {selected.id === 'Navigation'
            ? 'Ships sail through connected sea tiles. Navigation adds a clockwise or counterclockwise shortcut around the edge to the next sea area. Unexplored regions must be explored before sailing farther.'
            : selected.description}
        </p>
        {#if selected.unlocks && buildingInfo[selected.unlocks]}<p>
            <strong>{selected.unlocks}:</strong>
            {buildingInfo[selected.unlocks].effect}
          </p>{/if}
        {#if unavailableReason(selected) && !selected.owned}<small>{unavailableReason(selected)}</small>{/if}
        {#if $session.error && !reference}<p class="inline-error" role="alert">{$session.error}</p>{/if}
      </div>
      {#if !reference}<div class="research-payment">
          {#if !choice && !selected.owned && selected.payments.length > 1}
            <PaymentPicker
              options={selected.payments.map((option) => ({
                payment: option.payment,
                disabled: !option.action,
              }))}
              value={selectedPayment?.payment ?? selected.payment}
              onChange={(payment) => {
                chosenPayment = { advance: selected!.id, payment };
              }}
              pending={$session.pending}
              label="Research payment"
            />
          {/if}
          <button
            class="primary"
            title={unavailableReason(selected) || (eventImminent ? 'Research triggers an event' : undefined)}
            aria-describedby={eventImminent ? 'research-event-notice' : undefined}
            disabled={!selectedAction || $session.pending}
            onclick={() => selectedAction && controller.submit(selectedAction)}
          >
            {$session.pending
              ? 'Confirming…'
              : selected.owned
                ? 'Researched'
                : `${borrowing ? 'Use' : choice?.advanceMode === 'paid' ? 'Choose' : 'Research'} ${selected.name}`}
            {#if !selected.owned}<span
                >{#if eventImminent}<ScrollText size={13} aria-hidden="true" />{/if}{#if borrowing}Until end
                  of turn{:else if freeResearch}Free{:else if choice}Pay research cost next{:else}{#if selectedPayment}Pay
                    <ResourceAmount pile={selectedPayment.payment} compact /> ·
                  {/if}1 action{/if}</span
              >{/if}
          </button>
        </div>{/if}
      <button class="icon-button" aria-label="Close research details" onclick={() => selectAdvance(null)}
        ><X size={18} /></button
      >
    </section>
  {:else}<div class="research-legend">
      <span><Check size={14} /> Researched</span><span class="available-key">Available</span><span
        ><LockKeyhole size={13} /> Unavailable</span
      ><span
        >{reference
          ? 'Select an advance for its full rules.'
          : 'Select an advance for its full rules and confirmation.'}</span
      >
    </div>{/if}
</dialog>
