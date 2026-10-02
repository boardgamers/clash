<script lang="ts">
  import { tick } from 'svelte';
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
  import { resourceNames, type Resource, type Pile, type AdvanceView, type PublicAdvance } from './types';
  import { actionReason, pileText } from './model';
  import { groupIcons, researchPresentation } from './research';
  import { researchDecision } from './decision-controls';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const choice = $derived(researchDecision($session.view) ? $session.view!.decision : null);
  const freeResearch = $derived(!!choice && choice.advanceMode === 'free');
  const borrowing = $derived(choice?.advanceMode === 'borrow');
  let query = $state('');
  let category = $state('All');
  let researchedOnly = $state(false);
  const statusFilter = $derived(researchedOnly ? 'owned' : $session.availableOnly ? 'available' : 'all');
  function setStatusFilter(value: string) {
    researchedOnly = value === 'owned';
    if (!researchedOnly) controller.setAvailableOnly(value === 'available');
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
  let advances = $derived([...($session.view?.advances ?? [])].sort((a, b) => a.order - b.order));
  const availableAdvances = $derived(
    advances.filter((a) => (researchedOnly ? a.owned : !$session.availableOnly || !!a.action)),
  );
  let player = $derived($session.view?.players.find((p) => p.index === $session.seat));
  let civilizationAdvances = $derived(player?.civilizationAdvances ?? []);
  let visibleCivilizationAdvances = $derived(
    category === 'All' || category === 'Civilization'
      ? civilizationAdvances.filter(
          (a) =>
            (researchedOnly
              ? a.owned
              : !$session.availableOnly ||
                (!a.owned &&
                  a.prerequisites.some((p) => availableAdvances.some((advance) => advance.id === p.id)))) &&
            (matches(a) ||
              a.requirement.toLowerCase().includes(query.trim().toLowerCase()) ||
              a.prerequisites.some((p) => p.name.toLowerCase().includes(query.trim().toLowerCase()))),
        )
      : [],
  );
  let groups = $derived([...new Set(advances.map((a) => a.group))]);
  let selected = $derived(availableAdvances.find((a) => a.id === $session.selectedAdvance));
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
        availableAdvances.some((a) => a.group === group && matches(a)),
    ),
  );
  const selectedAction = $derived(choice ? selected?.action : selectedPayment?.action);
  function matches(advance: PublicAdvance) {
    return `${advance.name} ${advance.description} ${researchPresentation(advance).summary}`
      .toLowerCase()
      .includes(query.trim().toLowerCase());
  }
  function close() {
    if ($session.mode === 'research') controller.patch({ mode: 'overview', selectedAdvance: null });
  }
  function open(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
  async function showCivilizationAdvance(id: string) {
    setStatusFilter('all');
    query = '';
    category = 'Civilization';
    controller.patch({ selectedAdvance: null });
    await tick();
    document.getElementById(`civilization-${id}`)?.scrollIntoView({
      block: 'nearest',
      behavior: $session.reducedMotion ? 'instant' : 'smooth',
    });
  }
  async function prerequisite(id: string) {
    if (researchedOnly || !advances.find((a) => a.id === id)?.action) setStatusFilter('all');
    query = '';
    category = advances.find((a) => a.id === id)?.group ?? 'All';
    controller.patch({ selectedAdvance: id });
    await tick();
    document.getElementById(`research-${id}`)?.scrollIntoView({
      block: 'nearest',
      inline: 'nearest',
      behavior: $session.reducedMotion ? 'instant' : 'smooth',
    });
  }
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
  <header class="research-header">
    <div>
      <h2 id="research-title">
        {choice ? (choice.endOfAge ? 'Free advance' : choice.name) : 'Research'}
        {#if $session.view?.players.find((p) => p.index === $session.seat)}<EventMarkers
            remaining={$session.view.players.find((p) => p.index === $session.seat)!.eventTokens}
          />{/if}
      </h2>
      <p>
        {choice
          ? choice.description
          : 'Each advance scores ½ point. Branches unlock from their first advance.'}
      </p>
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
  {#if !choice}<ContextualCards {controller} context="research" />{/if}
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
    <button class:active={category === 'All'} onclick={() => (category = 'All')}>All advances</button>
    {#if player && civilizationAdvances.length}<button
        class:active={category === 'Civilization'}
        onclick={() => {
          category = 'Civilization';
          controller.patch({ selectedAdvance: null });
        }}><CivilizationEmblem civilization={player.civilization} size={16} />{player.civilization}</button
      >{/if}
    {#each groups as group}{@const Icon = groupIcons[group] ?? BookOpen}<button
        class:active={category === group}
        onclick={() => (category = group)}><Icon size={15} />{group}</button
      >{/each}
  </nav>
  <div class="research-tree" class:civilization-only={category === 'Civilization'} aria-label="Research tree">
    {#if player && visibleCivilizationAdvances.length}
      <CivilizationAdvances
        civilization={player.civilization}
        advances={visibleCivilizationAdvances}
        onPrerequisite={prerequisite}
      />
    {/if}
    {#each visibleGroups as group}{@const GroupIcon = groupIcons[group] ?? BookOpen}
      <section class="research-branch" aria-label={group}>
        <h3><GroupIcon size={18} />{group}</h3>
        {#each availableAdvances.filter((a) => a.group === group) as advance, index}{@const presentation =
            researchPresentation(advance)}{@const Icon = presentation.icon}{@const parent = advances.find(
            (a) => a.id === advance.required,
          )}
          <article
            id={`research-${advance.id}`}
            class="research-node"
            class:child={statusFilter === 'all' && index > 0}
            class:owned={advance.owned}
            class:available={!!advance.action}
            class:selected={selected?.id === advance.id}
            class:search-muted={!!query.trim() && !matches(advance)}
          >
            {#if index === 0 && parent}<button
                class="research-prerequisite"
                onclick={() => prerequisite(parent.id)}
                ><ArrowRight size={12} /> Requires {parent.name}</button
              >{/if}
            <button
              class="research-pick"
              aria-label={`${advance.name}: ${presentation.summary}`}
              aria-pressed={selected?.id === advance.id}
              title={advance.description}
              onclick={() => controller.patch({ selectedAdvance: advance.id })}
            >
              <span class="research-node-title"
                ><span class="advance-pictogram"><Icon size={21} /></span><strong>{advance.name}</strong
                >{#if advance.owned}<Check
                    size={16}
                    aria-label="Researched"
                  />{:else if !advance.action && (actionReason(advance.reason) || (parent && !parent.owned))}<LockKeyhole
                    size={13}
                    aria-label={!choice && parent && !parent.owned
                      ? `Needs ${parent.name}`
                      : actionReason(advance.reason)}
                  />{/if}</span
              >
              <span class="research-summary"><ResourceText text={presentation.summary} /></span>
              <span class="research-effects"
                >{#if advance.unlocks}<span title={`Unlocks ${advance.unlocks}`}
                    ><Hammer size={12} />{advance.unlocks}</span
                  >{/if}{#each Object.entries(advance.bonus ?? {}) as [resource, amount]}{@const BonusIcon =
                    resourceIcons[resource as Resource]}<span
                    title={`Research bonus: ${amount} ${resourceNames[resource as Resource]}`}
                    ><BonusIcon size={12} />+{amount} {resourceNames[resource as Resource]}</span
                  >{/each}{#each advance.bonusEffects ?? [] as bonus}<span
                    title={`Gain ${pileText(bonus.pile)} from ${bonus.source}`}
                  >
                    +<ResourceAmount pile={bonus.pile} compact /> · {bonus.source}
                  </span>{/each}</span
              >
              <span class="research-node-cost"
                >{#if advance.owned}<span class="researched-label">Researched</span>{:else}<span
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
                    >{advance.action
                      ? 'Available'
                      : !choice && parent && !advances.find((a) => a.id === parent.id)?.owned
                        ? `Needs ${parent.name}`
                        : actionReason(advance.reason)}</span
                  >{/if}</span
              >
            </button>
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
          : $session.availableOnly
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
          {selected.name}{#if selected.group === 'Seafaring'}<button
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
        {#if actionReason(selected.reason) && !selected.owned}<small>{actionReason(selected.reason)}</small
          >{/if}
        {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
      </div>
      <div class="research-payment">
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
          title={selected.reason ?? undefined}
          disabled={!selectedAction || $session.pending}
          onclick={() => selectedAction && controller.submit(selectedAction)}
        >
          {$session.pending
            ? 'Confirming…'
            : selected.owned
              ? 'Researched'
              : `${borrowing ? 'Use' : choice?.advanceMode === 'paid' ? 'Choose' : 'Research'} ${selected.name}`}
          {#if !selected.owned}<span
              >{#if borrowing}Until end of turn{:else if freeResearch}Free{:else if choice}Pay research cost
                next{:else}{#if selectedPayment}Pay <ResourceAmount pile={selectedPayment.payment} compact /> ·
                {/if}1 action{/if}</span
            >{/if}
        </button>
      </div>
      <button
        class="icon-button"
        aria-label="Close research details"
        onclick={() => controller.patch({ selectedAdvance: null })}><X size={18} /></button
      >
    </section>
  {:else}<div class="research-legend">
      <span><Check size={14} /> Researched</span><span class="available-key">Available</span><span
        ><LockKeyhole size={13} /> Unavailable</span
      ><span>Select an advance for its full rules and confirmation.</span>
    </div>{/if}
</dialog>
