<script lang="ts">
  import { Sparkles } from 'lucide-svelte';
  import { printedCardName } from './card-names';
  import ResourceText from './ResourceText.svelte';
  import type { EventNotice } from './event-notices';
  let { notice }: {notice: EventNotice} = $props();
</script>
<article class="event-explanation" aria-label={`${printedCardName(notice.name)} event`}>
  <h3><Sparkles size={18} />{printedCardName(notice.name)}</h3>
  {#each notice.rules as rule}<p><ResourceText text={rule} /></p>{/each}
</article>
<style>
  .event-explanation { display:block; width:100%; min-width:0; }
  h3 { display:flex; align-items:center; gap:.5rem; margin:0 0 .6rem; }
  p { margin:.5rem 0; line-height:1.5; font-size:.85rem; }
</style>
