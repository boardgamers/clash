<script lang="ts">
  import { Drama, ArrowRight, Dices } from 'lucide-svelte';
  import type { InfluenceContext } from './types';
  let { context }: { context: InfluenceContext } = $props();
  const step = $derived(context.stage === 'boost' ? 2 : 1);
</script>

<div class="influence-flow">
  <h2><Drama size={21} />Cultural influence</h2>
  <ol class="influence-steps" aria-label="Influence progress">
    {#each ['Target', 'Roll', 'Resolve'] as label, i}<li
        class:current={i === step}
        class:done={i < step}
        aria-current={i === step ? 'step' : undefined}
      >
        {label}
      </li>{/each}
  </ol>
  <div class="influence-route">
    {#if context.source}<span>From <b>{context.source}</b></span><ArrowRight size={15} />{/if}<span
      ><b>{context.name}</b> at <b>{context.target}</b></span
    >
  </div>
  {#if context.roll !== null}
    <div class="influence-roll">
      <Dices size={24} /><strong>{context.roll}</strong><span
        >Roll total · need {context.threshold}+{#if context.rollBonus}<small
            >Includes +{context.rollBonus} bonus</small
          >{/if}</span
      >
    </div>
  {/if}
  <p class="decision-description">
    {context.stage === 'range'
      ? 'Pay for the extra range, then roll. Any roll boost is a separate choice after you see the result.'
      : context.stage === 'boost'
        ? `Your roll is ${context.threshold - (context.roll ?? 0)} short. Spend culture to succeed, or accept the failed attempt.`
        : context.stage === 'reroll'
          ? 'Use your optional reroll, or keep this result and continue to any available boost.'
          : 'Pay the action fee to continue with this attempt. Range and roll boosts are shown separately.'}
  </p>
</div>
