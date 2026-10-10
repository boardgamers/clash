<script lang="ts">
  import { printedCardName } from './card-names';
  import { X, Layers, Swords, Zap } from 'lucide-svelte';
  import type { CardRule } from './card-reference';
  import { handCardDescription } from './card-text';
  import ResourceText from './ResourceText.svelte';
  let { card, onDismiss }: { card: CardRule; onDismiss: () => void } = $props();
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide cards-dialog"
  aria-label={`${printedCardName(card.name)} rules`}
  use:show
  onclose={onDismiss}
  onclick={(e) => {
    if (e.target === e.currentTarget) onDismiss();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onDismiss();
  }}
>
  <button class="close-guide icon-button" aria-label="Close card rules" onclick={onDismiss}
    ><X size={20} /></button
  >
  <h2><Layers size={23} />{printedCardName(card.name)}</h2>
  <p class="card-use-rule"><Zap size={14} />{card.free ? 'Free action' : 'Costs 1 action'}</p>
  <p><ResourceText text={handCardDescription(card.description)} /></p>
  {#if card.tactics}<section class="card-battle-use">
      <div class="card-use-label"><Swords size={15} />Or · Battle use</div>
      <h3>{printedCardName(card.tactics.name)}</h3>
      <p><ResourceText text={card.tactics.description} /></p>
    </section>{/if}
</dialog>
