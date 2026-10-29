<script lang="ts">
  import { Clock, Check, X, Sparkles } from 'lucide-svelte';
  import type { View } from './types';
  import type { Controller } from './controller';
  import RazeActions from './RazeActions.svelte';
  import ResourceText from './ResourceText.svelte';
  let { controller, offers, onclose }: { controller: Controller; offers: NonNullable<View['earlyChoices']>; onclose: () => void } = $props();
  const session = $derived(controller.session);
  let selections = $state<Record<string, string>>({});
  const raze = $derived(offers.length === 1 && offers[0].key.endsWith('/raze'));
  const first = $derived(offers.length === 1 && offers[0].key.endsWith(':first'));
  const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
</script>

<section class="action-panel floating-panel early-choices" class:raze-prompt={raze} aria-label={raze ? 'Raze city' : first ? 'Choose first player' : 'Choose ahead'}>
  <header>
    <h2>{#if raze}<Sparkles size={21} />Raze city{:else}<Clock size={18} />{first ? 'Choose first player' : 'Choose ahead'}{/if}</h2>
    <button class="icon-button" aria-label="Close early choices" onclick={onclose}><X size={18} /></button>
  </header>
    {#each offers as offer (offer.key)}
      {@const saved = offer.options.findIndex((o) => same(o.response, offer.selected))}
      {@const value = selections[offer.key] ?? (saved >= 0 ? JSON.stringify(offer.selected) : '')}
      {@const selected = offer.options.findIndex((o) => JSON.stringify(o.response) === value)}
      <div class="early-choice">
        {#if !raze && !first}<label for={'early-' + offer.key}>{offer.title}</label>{/if}
        {#if offer.key.endsWith('/advance')}
          <div class="choice-row">
            <button class="primary" disabled={$session.pending} onclick={() => controller.patch({mode: 'research', selectedAdvance: (offer.selected as {SelectAdvance?: string} | null)?.SelectAdvance ?? null})}>
              {saved >= 0 ? offer.options[saved].label : 'Choose an advance'}
            </button>
            {#if saved >= 0}<button disabled={$session.pending} onclick={() => controller.submit({QueueChoice:{key:offer.key,response:null}})}>Clear</button>{/if}
          </div>
          {#if saved >= 0}<p class="hint"><Check size={13} /> Saved</p>{/if}
        {:else if offer.key.endsWith('/raze')}
          <p class="hint">Select a highlighted city on the map.</p>
          <RazeActions queued selected={$session.earlyRaze?.selected ?? null} pending={$session.pending}
            onkeep={() => { controller.submit({QueueChoice:{key:offer.key,response:{SelectPositions:[]}}}); onclose(); }}
            onraze={() => { controller.submit({QueueChoice:{key:offer.key,response:{SelectPositions:[$session.earlyRaze!.selected]}}}); onclose(); }} />
          {#if saved >= 0}
            <div class="queued-choice">
              <span>Queued: {(offer.selected as {SelectPositions: string[]}).SelectPositions.length ? `raze ${(offer.selected as {SelectPositions: string[]}).SelectPositions[0]}` : 'keep all cities'}</span>
              <button disabled={$session.pending} onclick={() => { controller.submit({QueueChoice:{key:offer.key,response:null}}); onclose(); }}>Cancel queued choice</button>
            </div>
          {/if}
        {:else}
          <div class="option-list">
            {#each offer.options as option}
              <button class:chosen={same(option.response, offer.selected)} disabled={$session.pending}
                onclick={() => { controller.submit({QueueChoice:{key:offer.key,response:option.response}}); onclose(); }}>
                {first ? 'Queue ' : ''}{option.label}{#if same(option.response, offer.selected)}<Check size={15} />{/if}
              </button>
            {/each}
            {#if saved >= 0}<button disabled={$session.pending} onclick={() => { controller.submit({QueueChoice:{key:offer.key,response:null}}); onclose(); }}>Cancel queued choice</button>{/if}
          </div>
        {/if}
        {#if !offer.key.endsWith('/advance') && selected >= 0 && offer.options[selected]?.description}
          <p class="hint"><ResourceText text={offer.options[selected].description} /></p>
        {/if}
      </div>
    {/each}
</section>

<style>
  .early-choices { max-height: min(60vh, 32rem); overflow-y: auto; }
  header { display: flex; align-items: center; justify-content: space-between; gap: .75rem; }
  h2 { display: flex; align-items: center; gap: .5rem; margin: 0; }
  .hint { font-size: .8rem; color: var(--muted); line-height: 1.45; }
  .early-choice { margin-top: 1rem; }
  .raze-prompt .early-choice { margin-top: 0; }
  label { display: block; font-weight: 600; margin-bottom: .4rem; }
  .choice-row { display: flex; gap: .5rem; align-items: stretch; }
  .queued-choice { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--line); font-size: .75rem; color: var(--muted); }
  .queued-choice button { padding: 6px 8px; border: 1px solid var(--line); border-radius: 4px; color: var(--ink); font-size: inherit; }
  .option-list { display: grid; gap: .5rem; }
  .option-list button { display: flex; align-items: center; justify-content: space-between; text-align: left; padding: .7rem; border: 1px solid var(--line); border-radius: 4px; }
  .option-list button.chosen { border-color: var(--green); }

  .choice-row button { display: flex; align-items: center; gap: .3rem; white-space: nowrap; }
</style>
