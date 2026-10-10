<script lang="ts">
  import { Sparkles, X, ScrollText, Layers } from 'lucide-svelte';
  import type { ActiveEffect, View, EventInfo } from './types';
  import { printedCardName } from './card-names';
  import ResourceText from './ResourceText.svelte';
  import CardReferenceDialog from './CardReferenceDialog.svelte';
  import EventReferenceDialog from './EventReferenceDialog.svelte';
  import { effectSourceReference } from './active-effects';
  import type { CardRule } from './card-reference';
  let {
    effects,
    players,
    events,
    cards,
    onClose,
  }: {
    effects: ActiveEffect[];
    players: View['players'];
    events: EventInfo[];
    cards: CardRule[];
    onClose: () => void;
  } = $props();
  let reference = $state<ReturnType<typeof effectSourceReference>>(null);
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide active-effects-dialog"
  aria-label="Active effects"
  use:show
  onclose={onClose}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onClose();
  }}
>
  <button class="close-guide icon-button" aria-label="Close active effects" onclick={onClose}
    ><X size={20} /></button
  >
  <h2><Sparkles size={23} /> Active effects</h2>
  <div class="active-effects-list">
    {#each effects as effect}
      <section class="active-effect-rule">
        <h3>{printedCardName(effect.name)}</h3>
        {#if effect.source}{@const source = effectSourceReference(effect, events, cards)}
          <div class="effect-source">
            {#if effect.source.kind === 'Event'}<ScrollText size={14} />{:else}<Layers size={14} />{/if}
            <span>{effect.source.kind}</span><span aria-hidden="true">·</span>
            {#if source}<button
                class="journal-research-link"
                title={`View ${printedCardName(effect.source.name)} rules`}
                onclick={() => (reference = source)}
                >{printedCardName(
                  effect.source.name,
                )}{#if effect.source.kind === 'Event' && effect.source.id != null}
                  {' '}(E{effect.source.id}){/if}</button
              >{:else}<span>{printedCardName(effect.source.name)}</span>{/if}
          </div>{/if}
        <small
          >{effect.players.length
            ? effect.players
                .map(
                  (index) => players.find((player) => player.index === index)?.civilization ?? effect.scope,
                )
                .join(' · ')
            : effect.scope}</small
        >
        {#each effect.rules as rule}<p><ResourceText text={rule} /></p>{/each}
      </section>
    {:else}<p>No active effects.</p>{/each}
  </div>
</dialog>
{#if reference?.kind === 'event'}<EventReferenceDialog
    event={reference.event}
    onClose={() => (reference = null)}
  />{:else if reference?.kind === 'card'}<CardReferenceDialog
    card={reference.card}
    onDismiss={() => (reference = null)}
  />{/if}

<style>
  .active-effects-list {
    display: grid;
    gap: 1.25rem;
  }
  .active-effect-rule {
    padding: 1.1rem;
    border: 1px solid var(--border, #3b5564);
    border-radius: 8px;
  }
  .effect-source {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin: 0.4rem 0 0.55rem;
    color: var(--muted, #9ab8c7);
    font-size: 0.8rem;
  }
  .active-effect-rule h3 {
    margin: 0 0 0.3rem;
  }
  .active-effect-rule small {
    color: var(--muted, #9ab8c7);
  }
  .active-effect-rule p {
    margin: 0.7rem 0 0;
    line-height: 1.6;
  }
</style>
