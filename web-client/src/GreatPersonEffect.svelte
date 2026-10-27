<script lang="ts">
  import { Zap } from 'lucide-svelte';
  import ResourceText from './ResourceText.svelte';
  import { handCardDescription } from './card-text';
  let { card }: {card: {description: string; free?: boolean}} = $props();
</script>
<div class="great-person-effect">
  <span class="event-card-use">{#if card.description.startsWith('You make place')}Active when taken{:else}Play <span aria-label={`${card.free ? 0 : 1} actions`}>{card.free ? 0 : 1} <Zap size={13} /></span>{/if}</span>
  <ResourceText text={handCardDescription(card.description)} />
</div>
