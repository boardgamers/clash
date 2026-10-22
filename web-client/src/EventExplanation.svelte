<script lang="ts">
  import { handCardDescription } from './card-text';
  import { Sparkles } from 'lucide-svelte';
  import { printedCardName } from './card-names';
  import ResourceText from './ResourceText.svelte';
  import type { EventNotice } from './event-notices';
  let { notice }: {notice: EventNotice} = $props();
</script>
<article class="event-explanation" aria-label={`${printedCardName(notice.name)} event`}>
  <h3><Sparkles size={18} />{printedCardName(notice.name)}</h3>
  {#each notice.rules as rule}
    <p><ResourceText text={handCardDescription(rule)} /></p>
    {#if handCardDescription(rule) !== rule}
      <details><summary>Acquiring the card</summary><p><ResourceText text={rule.slice(0, rule.length - handCardDescription(rule).length).replace(/Action card: $/, '').replace(/: $/, '.')} /></p></details>
    {/if}
  {/each}
</article>
<style>
  .event-explanation { display:block; width:100%; min-width:0; }
  h3 { display:flex; align-items:center; gap:.5rem; margin:0 0 .6rem; }
  p { margin:.5rem 0; line-height:1.5; font-size:.85rem; }
</style>
