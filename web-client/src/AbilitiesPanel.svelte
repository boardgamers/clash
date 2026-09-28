<script lang="ts">
  import { X, Sparkles, Drama, ArrowRight } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceText from './ResourceText.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  let chosen = $state<number | null>(null);
  const target = $derived(chosen === null ? null : $session.view?.influence?.[chosen]);
</script>

<section class="action-panel floating-panel abilities-panel" aria-label="Abilities and influence">
  <button
    class="icon-button close-action"
    aria-label="Close abilities"
    onclick={() => controller.patch({ abilitiesOpen: false })}><X size={18} /></button
  >
  <h2><Sparkles size={21} />Abilities</h2>
  {#each $session.view?.specialActions ?? [] as action}<details class="ability-offer">
      <summary
        >{action.name}{#if action.position}
          · {action.position}{/if}</summary
      >
      <p><ResourceText text={action.description} /></p>
      <button
        class="primary wide"
        disabled={$session.pending}
        onclick={() => controller.submit(action.action)}>Use {action.name}</button
      >
    </details>{:else}<p class="settler-empty">No special actions available.</p>{/each}
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
        onclick={() => (chosen = i)}>{offer.position} · {offer.name}<small>{offer.variant}</small></button
      >
    {:else}<p class="settler-empty">No eligible targets.</p>{/each}
  </div>
  {#if target}{#if Object.values(target.payment).some(Boolean)}<ResourceAmount
        pile={target.payment}
      />{/if}<button
      class="primary wide"
      disabled={$session.pending}
      onclick={() => controller.submit(target.action)}>Attempt influence<ArrowRight size={16} /></button
    >{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
