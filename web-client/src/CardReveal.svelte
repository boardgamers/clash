<script lang="ts">
  import { X, ArrowRight, Target, Landmark } from 'lucide-svelte';
  import WonderCard from './WonderCard.svelte';
  import ObjectiveCondition from './ObjectiveCondition.svelte';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let draw = $derived($session.cardDraws[0]);
  function dismiss() {
    controller.patch({ cardDraws: $session.cardDraws.slice(1) });
  }
  function openHand() {
    controller.patch({
      wondersOpen: draw.kind === 'wonder',
      objectivesOpen: draw.kind === 'objective',
      cardDraws: [],
    });
  }
</script>

{#if draw}
  {#key `${draw.kind}-${draw.card.id}`}
    <aside class="card-reveal" class:still={$session.reducedMotion} aria-label="New card" aria-live="polite">
      <div class="reveal-title">
        <span
          >{#if draw.kind === 'wonder'}<Landmark size={16} />{:else}<Target size={16} />{/if} You drew {draw.kind ===
          'wonder'
            ? 'a wonder'
            : 'an objective'}</span
        ><button class="icon-button" aria-label="Dismiss drawn card" onclick={dismiss}><X size={17} /></button
        >
      </div>
      {#if draw.kind === 'wonder'}<WonderCard card={draw.card} />{:else}
        <div class="drawn-objective">
          {#each draw.card.objectives as objective, i}{#if i}<span class="objective-divider">or</span>{/if}
            <ObjectiveCondition {objective} />{/each}
        </div>
      {/if}
      <button class="reveal-open" onclick={openHand}
        >View {draw.kind === 'wonder' ? 'wonders' : 'objectives'} <ArrowRight size={16} /></button
      >
      {#if $session.cardDraws.length > 1}<button class="reveal-next" onclick={dismiss}
          >Next card · {$session.cardDraws.length - 1} more</button
        >{/if}
    </aside>
  {/key}
{/if}
