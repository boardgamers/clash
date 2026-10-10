<script lang="ts">
  import { Sparkles, X } from 'lucide-svelte';
  import type { ActiveEffect, View } from './types';
  import { printedCardName } from './card-names';
  import ResourceText from './ResourceText.svelte';
  let {
    effects,
    players,
    onClose,
  }: { effects: ActiveEffect[]; players: View['players']; onClose: () => void } = $props();
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
  {#each effects as effect}
    <section class="active-effect-rule">
      <h3>{printedCardName(effect.name)}</h3>
      <small
        >{effect.players.length
          ? effect.players
              .map((index) => players.find((player) => player.index === index)?.civilization ?? effect.scope)
              .join(' · ')
          : effect.scope}</small
      >
      {#each effect.rules as rule}<p><ResourceText text={rule} /></p>{/each}
    </section>
  {:else}<p>No active effects.</p>{/each}
</dialog>

<style>
  .active-effect-rule {
    padding: 1.1rem 0;
    border-top: 1px solid var(--border, #3b5564);
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
