<script lang="ts">
  import { X } from 'lucide-svelte';
  import { printedCardName } from './card-names';
  import type { WonderCard as Card } from './types';
  import WonderCard from './WonderCard.svelte';
  let { card, onClose }: { card: Card; onClose: () => void } = $props();
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide"
  aria-label={`${printedCardName(card.name)} rules`}
  use:show
  onclose={onClose}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onClose();
  }}
>
  <button class="close-guide icon-button" aria-label="Close card rules" onclick={onClose}
    ><X size={20} /></button
  >
  <WonderCard {card} />
</dialog>
