<script lang="ts">
  import { X, Landmark, Hammer, BookOpen, Layers } from 'lucide-svelte';
  import WonderCard from './WonderCard.svelte';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let tab = $state<'hand' | 'reference'>('hand');
  const close = () => controller.patch({ wondersOpen: false });
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide wonders-dialog"
  aria-labelledby="wonders-title"
  use:show
  onclose={close}
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
>
  <button class="close-guide icon-button" aria-label="Close wonders" onclick={close}><X size={20} /></button>
  <h2 id="wonders-title">Wonders</h2>
  <nav class="civilization-tabs" aria-label="Wonder views">
    <button class:active={tab === 'hand'} aria-pressed={tab === 'hand'} onclick={() => (tab = 'hand')}
      ><Layers size={16} />Your hand <span>{$session.view?.wonderCards.length ?? 0}</span></button
    >
    <button
      class:active={tab === 'reference'}
      aria-pressed={tab === 'reference'}
      onclick={() => (tab = 'reference')}><BookOpen size={16} />All wonders</button
    >
  </nav>
  {#if tab === 'reference'}
    <p class="hand-note">
      Reference · You need the card in hand, a happy city, Engineering, and the required advance to construct
      a wonder.
    </p>
    {#each $session.view?.wonderCatalog ?? [] as card (card.id)}
      <WonderCard {card} />
    {/each}
  {:else}
    <p class="guide-intro">
      Private hand · {$session.view?.wonderCards.length ?? 0}
      {$session.view?.wonderCards.length === 1 ? 'card' : 'cards'}
    </p>
    {#each $session.view?.wonderCards ?? [] as card (card.id)}
      <div class="wonder-hand-entry">
        <WonderCard {card} />
        <button
          class="primary wide"
          disabled={!card.action || $session.pending}
          title={card.reason ?? 'Choose a city and pay to construct this wonder'}
          onclick={() => card.action && controller.submit(card.action)}
          ><Hammer size={16} />Construct {card.name}</button
        >
      </div>
    {:else}
      <div class="hand-empty">
        <Landmark size={32} />
        <p>No wonder cards yet.</p>
        <span>Research Engineering to draw your first wonder card.</span>
      </div>
    {/each}
    {#if $session.view?.wonderCards.length}
      <p class="hand-note">
        A card in hand scores no points. Construct the wonder in a happy city with Engineering and its
        required advance to gain its effects and points.
      </p>
    {/if}
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</dialog>
