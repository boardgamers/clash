<script lang="ts">
  import { X, Layers, Zap, Swords } from 'lucide-svelte';
  import ResourceText from './ResourceText.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import type { Controller } from './controller';
  import { actionReason } from './model';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const close = () => controller.patch({ cardsOpen: false });
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide cards-dialog"
  aria-labelledby="action-cards-title"
  use:show
  onclose={close}
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
>
  <button class="close-guide icon-button" aria-label="Close action cards" onclick={close}
    ><X size={20} /></button
  >
  <h2 id="action-cards-title"><Layers size={23} />Action cards</h2>
  {#if $session.view?.actionCards?.some((card) => card.tactics)}
    <p class="card-use-rule">
      Choose one use, then discard. Battle uses require Tactics: one card per combat round.
    </p>
  {/if}
  {#each $session.view?.actionCards ?? [] as card}
    <article class="play-card">
      <h3>{card.name}</h3>
      <p><ResourceText text={card.description} /></p>
      <button
        class="primary wide card-play-button"
        title={card.reason ?? 'Play this card'}
        disabled={!card.action || $session.pending}
        onclick={() => card.action && controller.submit(card.action, card.cost)}
        ><span>Play {card.name}</span>
        <span class="card-play-cost"
          >{#if card.cost && Object.values(card.cost).some(Boolean)}<ResourceAmount pile={card.cost} />{/if}
          <span
            aria-label={card.free ? 'Free action' : 'Costs 1 action'}
            title={card.free ? 'Free action' : 'Costs 1 action'}><Zap size={13} />{card.free ? 0 : 1}</span
          ></span
        ></button
      >
      {#if actionReason(card.reason)}<small>{actionReason(card.reason)}</small>{/if}
      {#if card.tactics}<section class="card-battle-use" aria-label={`Battle use: ${card.tactics.name}`}>
          <div class="card-use-label"><Swords size={15} />Or · Battle use</div>
          <h4>{card.tactics.name}</h4>
          <p><ResourceText text={card.tactics.description} /></p>
        </section>{/if}
    </article>
  {:else}<p>No action cards in hand.</p>{/each}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</dialog>
