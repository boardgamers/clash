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
    Hammer,
    BookOpen,
  } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { resourceNames, type Resource, type AdvanceView } from './types';
  import { pileText } from './model';
  import { groupIcons, researchPresentation } from './research';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let query = $state('');
  let category = $state('All');
  const resourceIcons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
  };
  let advances = $derived([...($session.view?.advances ?? [])].sort((a, b) => a.order - b.order));
  let groups = $derived([...new Set(advances.map((a) => a.group))]);
  let selected = $derived(advances.find((a) => a.id === $session.selectedAdvance));
  let visibleGroups = $derived(
    groups.filter(
      (group) =>
        (category === 'All' || group === category) && advances.some((a) => a.group === group && matches(a)),
    ),
  );
  function matches(advance: AdvanceView) {
    return `${advance.name} ${advance.description} ${researchPresentation(advance).summary}`
      .toLowerCase()
      .includes(query.trim().toLowerCase());
  }
  function close() {
    controller.patch({ mode: 'overview', selectedAdvance: null });
  }
  function open(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
  async function prerequisite(id: string) {
    query = '';
    category = 'All';
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
      <h2 id="research-title">Research</h2>
      <p>Each advance scores ½ point. Branches unlock from their first advance.</p>
    </div>
    <label class="research-search"
      ><Search size={17} /><input
        aria-label="Search research"
        placeholder="Find an effect or advance…"
        bind:value={query}
      /></label
    >
    <button class="icon-button" aria-label="Close research" onclick={close}><X size={21} /></button>
  </header>
  <nav class="research-filters" aria-label="Research categories">
    <button class:active={category === 'All'} onclick={() => (category = 'All')}>All advances</button>
    {#each groups as group}{@const Icon = groupIcons[group] ?? BookOpen}<button
        class:active={category === group}
        onclick={() => (category = group)}><Icon size={15} />{group}</button
      >{/each}
  </nav>
  <div class="research-tree" aria-label="Research tree">
    {#each visibleGroups as group}{@const GroupIcon = groupIcons[group] ?? BookOpen}
      <section class="research-branch" aria-label={group}>
        <h3><GroupIcon size={18} />{group}</h3>
        {#each advances.filter((a) => a.group === group) as advance, index}{@const presentation =
            researchPresentation(advance)}{@const Icon = presentation.icon}{@const parent = advances.find(
            (a) => a.id === advance.required,
          )}
          <article
            id={`research-${advance.id}`}
            class="research-node"
            class:child={index > 0}
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
                  />{:else if !advance.action}<LockKeyhole
                    size={13}
                    aria-label={advance.reason ?? 'Unavailable'}
                  />{/if}</span
              >
              <span class="research-summary">{presentation.summary}</span>
              <span class="research-effects"
                >{#if advance.unlocks}<span title={`Unlocks ${advance.unlocks}`}
                    ><Hammer size={12} />{advance.unlocks}</span
                  >{/if}{#each Object.entries(advance.bonus ?? {}) as [resource, amount]}{@const BonusIcon =
                    resourceIcons[resource as Resource]}<span
                    title={`Research bonus: ${amount} ${resourceNames[resource as Resource]}`}
                    ><BonusIcon size={12} />+{amount} {resourceNames[resource as Resource]}</span
                  >{/each}</span
              >
              <span class="research-node-cost"
                >{#if advance.owned}<span class="researched-label">Researched</span
                  >{:else}{#each Object.entries(advance.payment) as [resource, amount]}{@const CostIcon =
                      resourceIcons[resource as Resource]}<span
                      ><CostIcon size={13} />{amount} {resourceNames[resource as Resource]}</span
                    >{/each}{#if !Object.values(advance.payment).some(Boolean)}<span>No resources</span
                    >{/if}<span class="research-availability"
                    >{advance.action
                      ? 'Available'
                      : parent && !advances.find((a) => a.id === parent.id)?.owned
                        ? `Needs ${parent.name}`
                        : advance.reason}</span
                  >{/if}</span
              >
            </button>
          </article>
        {/each}
      </section>
    {:else}<p class="research-empty">No advances match “{query}”.</p>{/each}
  </div>
  {#if selected}
    <section class="research-detail" aria-label="Research details">
      <div>
        <h3>{selected.name}</h3>
        <p>{selected.description}</p>
        {#if selected.bonus && Object.values(selected.bonus).some(Boolean)}<small
            >Research bonus: {pileText(selected.bonus)}</small
          >{/if}{#if selected.reason && !selected.owned}<small>{selected.reason}</small>{/if}
        {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
      </div>
      <button
        class="primary"
        disabled={!selected.action || $session.pending}
        onclick={() => selected?.action && controller.submit(selected.action)}
        >{$session.pending ? 'Confirming…' : selected.owned ? 'Researched' : `Research ${selected.name}`}
        <span>{selected.owned ? '' : `${pileText(selected.payment)} · 1 action`}</span></button
      >
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
