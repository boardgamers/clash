<script lang="ts">
  import { printedCardName, printedCardText } from './card-names';
  import { Check, Layers, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { contextualCards, activeCollectionCard, type CardContext } from './contextual-cards';
  import ResourceAmount from './ResourceAmount.svelte';
  let { controller, context }: { controller: Controller; context: CardContext } = $props();
  const session = $derived(controller.session);
  const offers = $derived(contextualCards($session.view, context));
  const active = $derived(context === 'collect' ? activeCollectionCard($session.game) : null);
</script>

{#if offers.length || active}
  <div
    class="contextual-cards"
    class:after-battle-cards={context === 'after-battle'}
    role="group"
    aria-label={context === 'after-battle' ? 'Cards available after battle' : 'Useful action cards'}
  >
    {#if active}<span class="contextual-card-active"><Check size={14} />{active}</span>{/if}
    {#each offers as { card, benefit } (card.id)}
      <button
        class="secondary contextual-card"
        disabled={$session.pending}
        title={`${printedCardText(card.description)}\nDiscard this card after use.${card.tactics ? ` Also gives up ${printedCardName(card.tactics.name)}, its battle effect.` : ''}`}
        onclick={() => controller.playContextualCard(card.id, context)}
      >
        <Layers size={14} /><span
          >{context === 'after-battle'
            ? printedCardName(card.name)
            : `Play ${printedCardName(card.name)}`}</span
        ><strong>{benefit}</strong>
        {#if offers.filter((o) => o.card.name === card.name).length > 1 && card.tactics}
          <small>({printedCardName(card.tactics.name)})</small>
        {/if}
        {#if Object.values(card.cost).some(Boolean)}<ResourceAmount pile={card.cost} />{/if}
        {#if !card.free}<span class="contextual-card-cost"><Zap size={12} />1</span>{/if}
      </button>
    {/each}
  </div>
{/if}
