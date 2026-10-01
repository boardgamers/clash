<script lang="ts">
  import {
    X,
    Sparkles,
    Drama,
    ArrowRight,
    Footprints,
    Landmark,
    RotateCcw,
    Zap,
    ArrowLeft,
    MapPin,
  } from 'lucide-svelte';
  import { activeCityAbility, groupAbilities } from './abilities';
  import CityFacts from './CityFacts.svelte';
  import type { Controller } from './controller';
  import ResourceText from './ResourceText.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  const groups = $derived(groupAbilities($session.view?.specialActions));
  const active = $derived(activeCityAbility($session));
  const selected = $derived(active?.offers.find((offer) => offer.position === $session.abilityCity));
  const selectedCity = $derived($session.view?.cities.find((city) => city.position === selected?.position));
  let chosen = $state<number | null>(null);
  let chosenOrigin = $state<string | null>(null);
  const target = $derived(chosen === null ? null : $session.view?.influence?.[chosen]);
  const origin = $derived(
    target?.origins?.find((o) => o.position === (chosenOrigin ?? target.origin)) ?? target?.origins?.[0],
  );
</script>

<section
  class="action-panel floating-panel abilities-panel"
  class:board-ability={!!active}
  class:selection-tray={!!active}
  aria-label="Abilities and influence"
>
  {#if active}
    <header class="movement-heading">
      <button
        class="icon-button"
        aria-label="Back to abilities"
        onclick={() => controller.patch({ abilityChoice: null, abilityCity: null })}
        ><ArrowLeft size={17} /></button
      >
      <h2><Sparkles size={18} />{active.offer.name}</h2>
      <button
        class="icon-button close-movement"
        aria-label="Close abilities"
        onclick={() => controller.patch({ abilitiesOpen: false })}><X size={18} /></button
      >
    </header>
    <p class="decision-description"><ResourceText text={active.offer.description} /></p>
    <div class="ability-map-confirm">
      {#if selectedCity}<span class="ability-selected-city"
          ><MapPin size={15} />Selected city <CityFacts
            size={selectedCity.size}
            mood={selectedCity.mood}
          /></span
        >
      {:else}<span class="movement-hint">Choose a highlighted city.</span>{/if}
      <button
        class="primary"
        disabled={!selected || $session.pending}
        onclick={() => selected && controller.submit(selected.action, selected.cost)}
      >
        Use {active.offer.name}
        {#if selected?.cost && Object.values(selected.cost).some(Boolean)}<ResourceAmount
            pile={selected.cost}
            compact
          />{/if}
        {#if active.offer.free}<small>Free action</small>{:else if active.offer.free === false}<span
            class="ability-cost"><Zap size={14} />1</span
          >{/if}
      </button>
    </div>
    {#if selected?.activatesCity && selectedCity && selectedCity.activations > 0}<ActivationStatus
        city={selectedCity}
        warning
      />{/if}
  {:else}
    <button
      class="icon-button close-action"
      aria-label="Close abilities"
      onclick={() => controller.patch({ abilitiesOpen: false })}><X size={18} /></button
    >
    <h2><Sparkles size={21} />Abilities</h2>
    {#each groups as group}
      {@const action = group.offer}
      {@const city = $session.view?.cities.find((c) => c.position === action.activatesCity)}
      <article class="ability-offer">
        <h3>{action.name}</h3>
        <p><ResourceText text={action.description} /></p>
        {#if city && city.activations > 0}<ActivationStatus {city} warning />{/if}
        <button
          class="primary wide"
          disabled={$session.pending}
          onclick={() =>
            action.position
              ? controller.chooseAbility(group.key)
              : controller.submit(action.action, action.cost)}
          >{action.position ? 'Choose city' : `Use ${action.name}`}
          <span class="ability-cost">
            {#if action.cost && Object.values(action.cost).some(Boolean)}<ResourceAmount
                pile={action.cost}
                compact
              />{/if}
            {#if action.activatesCity}<small>+ research</small>{/if}
            {#if action.free}<small>Free action</small>{:else if action.free === false}<Zap size={14} />1{/if}
          </span></button
        >
      </article>{:else}<p class="settler-empty">No special actions available.</p>{/each}
    <h3><Drama size={18} />Cultural influence</h3>
    <div class="decision-options">
      {#each $session.view?.influence ?? [] as offer, i}<button
          class:selected={chosen === i}
          title={`${offer.variant} · From ${offer.origin}`}
          aria-pressed={chosen === i}
          onmouseenter={() => onHighlight(offer.position)}
          onmouseleave={() => onHighlight(null)}
          onfocus={() => onHighlight(offer.position)}
          onblur={() => onHighlight(null)}
          onclick={() => {
            chosen = i;
            chosenOrigin = null;
          }}>{offer.position} · {offer.name}<small>{offer.variant}</small></button
        >
      {:else}<p class="settler-empty">No eligible targets.</p>{/each}
    </div>
    {#if target}
      {#if target.origins?.length}<div class="influence-origins" aria-label="Influence from">
          <span>From</span>
          {#each target.origins as from}<button
              class:selected={origin === from}
              aria-pressed={origin === from}
              title={from.reroll ? 'Buddhism · One reroll available' : from.settlers ? 'Settlers' : 'City'}
              onmouseenter={() => onHighlight(from.position)}
              onmouseleave={() => onHighlight(null)}
              onfocus={() => onHighlight(from.position)}
              onblur={() => onHighlight(null)}
              onclick={() => (chosenOrigin = from.position)}
            >
              {#if from.settlers}<Footprints size={15} />{:else}<Landmark size={15} />{/if}
              {from.position}{#if from.reroll}<RotateCcw size={14} />{/if}
            </button>{/each}
        </div>{/if}
      {#if Object.values(origin?.payment ?? target.payment).some(Boolean)}<ResourceAmount
          pile={origin?.payment ?? target.payment}
        />{/if}<button
        class="primary wide"
        disabled={$session.pending}
        onclick={() => controller.submit(origin?.action ?? target.action)}
        >Attempt influence<ArrowRight size={16} /></button
      >{/if}
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
