<script lang="ts">
  import { X, ArrowRight } from 'lucide-svelte';
  import type { Controller } from './controller';
  import EventExplanation from './EventExplanation.svelte';
  let {controller}: {controller: Controller} = $props();
  const session = $derived(controller.session);
  const notice = $derived($session.eventNotices?.[0]);
  function dismiss() {controller.patch({eventNotices:$session.eventNotices?.slice(1)});}
</script>
{#if notice && !$session.playback}
  <aside class="card-reveal" class:still={$session.reducedMotion} aria-label="Triggered event" aria-live="polite">
    <div class="reveal-title"><span>Game event</span><button class="icon-button" aria-label="Dismiss event explanation" onclick={dismiss}><X size={18}/></button></div>
    <EventExplanation {notice}/>
    {#if ($session.eventNotices?.length ?? 0)>1}<button class="reveal-next" onclick={dismiss}>Next event <ArrowRight size={15}/></button>{/if}
  </aside>
{/if}
