<script lang="ts">
  import { ScrollText, X } from 'lucide-svelte';
  import { printedCardName } from './card-names';
  import type { EventInfo } from './types';
  import ResourceText from './ResourceText.svelte';
  let { event, onClose }: { event: EventInfo; onClose: () => void } = $props();
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide"
  aria-label={`${printedCardName(event.name)} rules`}
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
  <h2><ScrollText size={23} />{printedCardName(event.name)}</h2>
  <p class="tiny-label">Event · E{event.id}</p>
  {#each event.rules as rule}<p><ResourceText text={rule} /></p>{/each}
</dialog>
