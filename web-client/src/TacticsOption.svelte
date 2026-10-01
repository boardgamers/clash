<script lang="ts">
  import { Check, Swords, ChevronRight } from 'lucide-svelte';
  import type { Decision } from './types';
  import ResourceText from './ResourceText.svelte';
  let {
    option,
    selected,
    pending,
    id,
    onSelect,
  }: {
    option: Decision['options'][number];
    selected: boolean;
    pending: boolean;
    id: string;
    onSelect: () => void;
  } = $props();
  const card = $derived(option.card?.kind === 'action' ? option.card : null);
</script>

{#if card?.tactics}
  <article class="tactics-choice" class:selected>
    <button
      class="decision-option tactics-option"
      class:selected
      aria-label={`Select ${card.tactics.name}`}
      aria-describedby={`${id}-battle`}
      aria-pressed={selected}
      disabled={pending}
      onclick={onSelect}
    >
      <span class="decision-option-content">
        <strong><Swords size={15} />{card.tactics.name}</strong>
        <span class="decision-rule" id={`${id}-battle`}><ResourceText text={card.tactics.description} /></span
        >
      </span>
      <span class="decision-selection" aria-hidden="true"
        >{#if selected}<Check size={13} />{/if}</span
      >
    </button>
    <details class="tactics-civil">
      <summary><ChevronRight size={13} />Civil action · {card.name}</summary>
      <div>
        <small>Unavailable during battle</small>
        <p><ResourceText text={card.description} /></p>
      </div>
    </details>
  </article>
{/if}
