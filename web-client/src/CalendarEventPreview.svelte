<script lang="ts">
  import { ArrowDownToLine, ArrowUpToLine } from 'lucide-svelte';
  import type { Decision, Move, Pile } from './types';
  import { printedCardName } from './card-names';
  import GreatPersonAcquisition from './GreatPersonAcquisition.svelte';
  import GreatPersonEffect from './GreatPersonEffect.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';

  let {
    preview,
    choices,
    pending,
    onSubmit,
  }: {
    preview: NonNullable<Decision['calendarPreview']>;
    choices: { payment: Pile; action: Move | null }[];
    pending: boolean;
    onSubmit: (action: Move) => void;
  } = $props();
</script>

<section class="calendar-preview" aria-label="Next event">
  <header class="calendar-event-heading">
    <span class="tiny-label">Next event</span>
    <h3>{printedCardName(preview.name)}</h3>
    <p class="calendar-preview-note">Preview only. This event has not been triggered.</p>
  </header>
  {#if preview.card}
    <section class="calendar-event-section">
      <h4>Civil effect</h4>
      <GreatPersonEffect card={preview.card} />
    </section>
  {/if}
  {#if preview.rules.length}
    <section class="calendar-event-section">
      <h4>Event effects</h4>
      {#each preview.rules as rule}<p><ResourceText text={rule} /></p>{/each}
    </section>
  {/if}
  {#if preview.card}
    <section class="calendar-event-section">
      <h4>Card acquisition</h4>
      <GreatPersonAcquisition />
    </section>
  {/if}
</section>
<div class="calendar-actions">
  {#each choices as choice}
    {@const paid = Object.values(choice.payment).some(Boolean)}
    <button
      class="secondary"
      disabled={pending || !choice.action}
      onclick={() => choice.action && onSubmit(choice.action)}
    >
      {#if paid}<ArrowDownToLine size={17} aria-hidden="true" />{:else}<ArrowUpToLine size={17} aria-hidden="true" />{/if}
      <span>{paid ? 'Put at the bottom' : 'Keep on top'}</span>
      {#if paid}<ResourceAmount pile={choice.payment} compact />{/if}
    </button>
  {/each}
</div>

<style>
  .calendar-preview {
    margin: 10px 0 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--paper2);
    overflow: hidden;
  }
  .calendar-event-heading,
  .calendar-event-section {
    padding: 12px 14px;
  }
  .calendar-event-heading h3 {
    margin: 3px 0 6px;
    font-size: 21px;
    line-height: 1.2;
  }
  .calendar-preview-note {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
    margin: 0;
  }
  .calendar-event-section {
    border-top: 1px solid var(--line);
    font-size: 13px;
    line-height: 1.55;
  }
  .calendar-event-section h4 {
    font-size: 12px;
    color: var(--muted);
    margin: 0 0 6px;
  }
  .calendar-event-section p,
  .calendar-event-section :global(.great-person-acquisition),
  .calendar-event-section :global(.great-person-effect) {
    margin: 0;
    line-height: inherit;
  }
  .calendar-event-section p + p {
    margin-top: 7px;
  }
  .calendar-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .calendar-actions button {
    flex: 1 1 180px;
    justify-content: center;
    min-height: 44px;
  }
  .calendar-actions button > span {
    text-align: start;
  }
  .calendar-actions button :global(svg) {
    flex-shrink: 0;
  }
</style>
