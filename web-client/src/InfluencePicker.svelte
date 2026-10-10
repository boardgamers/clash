<script lang="ts">
  import {
    Drama,
    ArrowRight,
    ArrowLeft,
    Landmark,
    Footprints,
    RotateCcw,
    Zap,
    X,
    TriangleAlert,
  } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { activeInfluence, influenceKey, influenceUpfrontCost, cheaperInfluenceOrigin } from './influence';
  import ResourceAmount from './ResourceAmount.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  const influence = $derived(activeInfluence($session));
  const offers = $derived(influence?.offers ?? []);
  const target = $derived(influence?.target ?? null);
  const origin = $derived(influence?.origin ?? null);
  const cheaper = $derived(cheaperInfluenceOrigin(target, origin));
  const attemptKey = $derived(`${target ? influenceKey(target) : ''}:${origin?.position}`);
  let confirmedSource = $state<string | null>(null);
  $effect(() => {
    attemptKey;
    confirmedSource = null;
  });
  function roll() {
    if (!target) return;
    if (cheaper && confirmedSource !== attemptKey) {
      confirmedSource = attemptKey;
      return;
    }
    controller.startInfluence(origin?.action ?? target.action, actionPayment, rangePayment);
  }
  const rangePayment = $derived(origin?.payment ?? target?.payment ?? {});
  const actionPayment = $derived(origin?.actionPayment ?? target?.actionPayment ?? {});
  const upfront = $derived(influenceUpfrontCost(actionPayment, rangePayment));
  // A city clicked on the map narrows the list to its own candidates.
  const groups = $derived(influence?.position ? [influence.position] : (influence?.targets ?? []));
  const bonus = $derived(origin?.rollBonus ?? target?.rollBonus ?? 0);
  const preventBoost = $derived(origin?.preventBoost ?? target?.preventBoost ?? false);
</script>

<header class="movement-heading">
  <button
    class="icon-button"
    aria-label="Back to abilities"
    onclick={() => controller.patch({ influenceMode: false })}><ArrowLeft size={17} /></button
  >
  <h2><Drama size={18} />Cultural influence</h2>
  <button
    class="icon-button close-movement"
    aria-label="Close abilities"
    onclick={() => controller.patch({ abilitiesOpen: false })}><X size={18} /></button
  >
</header>
<div class="influence-picker">
  <ol class="influence-steps" aria-label="Influence progress">
    <li class="current" aria-current="step">Target</li>
    <li>Roll</li>
    <li>Resolve</li>
  </ol>
  {#if !target}
    <p class="movement-hint">Click a highlighted city or building on the map, or choose below.</p>
    <div class="influence-targets">
      {#each groups as position}<div class="influence-target-group">
          <strong>At {position}</strong>
          <div class="decision-options">
            {#each offers.filter((o) => o.position === position) as offer}<button
                onmouseenter={() => onHighlight(offer.position)}
                onmouseleave={() => onHighlight(null)}
                onfocus={() => onHighlight(offer.position)}
                onblur={() => onHighlight(null)}
                onclick={() => {
                  controller.selectInfluenceTarget(influenceKey(offer));
                  onHighlight(null);
                }}>{offer.name}<small>{offer.variant}</small></button
              >{/each}
          </div>
        </div>{:else}<p class="settler-empty">No eligible targets.</p>{/each}
    </div>
  {/if}
  {#if target || influence?.position}<button
      class="secondary compact influence-change"
      onclick={() => {
        controller.patch({ influencePosition: null, influenceTarget: null, influenceOrigin: null });
        onHighlight(null);
      }}>Change target</button
    >{/if}
  {#if target}
    <div class="influence-preview">
      <div class="influence-route">
        <b>{origin?.position ?? target.origin}</b><ArrowRight size={15} /><span
          >{target.name} at <b>{target.position}</b></span
        >
      </div>
      {#if (target.origins?.length ?? 0) > 1}<div class="influence-origins" aria-label="Influence from">
          <span>From</span>{#each target.origins ?? [] as from}<button
              class:selected={origin === from}
              aria-pressed={origin === from}
              title={from.reroll ? 'Buddhism reroll available' : from.settlers ? 'Settlers' : 'City'}
              onmouseenter={() => onHighlight(from.position)}
              onmouseleave={() => onHighlight(null)}
              onfocus={() => onHighlight(from.position)}
              onblur={() => onHighlight(null)}
              onclick={() => controller.selectInfluenceOrigin(from.position)}
            >
              {#if from.settlers}<Footprints size={15} />{:else}<Landmark size={15} />{/if}{from.position}
              {#if Object.values(from.payment).some(Boolean)}<ResourceAmount
                  pile={from.payment}
                  compact
                />{:else}<small>In range</small>{/if}
              {#if from.reroll}<RotateCcw size={14} />{/if}
            </button>{/each}
        </div>{/if}
      <div class="influence-cost-line">
        <span>Action</span><span
          >{#if origin?.free ?? target.free}Free action{:else}<Zap size={14} />1 action{/if}{#if Object.values(actionPayment).some(Boolean)}<ResourceAmount
              pile={actionPayment}
              compact
            />{/if}</span
        >
      </div>
      <div class="influence-cost-line">
        <span>Extra range</span><span
          >{#if Object.values(rangePayment).some(Boolean)}<ResourceAmount
              pile={rangePayment}
              compact
            />{:else}None needed{/if}</span
        >
      </div>
      <p class="movement-hint">
        Roll {Math.max(1, 5 - bonus)}+{#if bonus}
          (+{bonus} bonus){/if} to succeed. {preventBoost
          ? 'This target prevents spending culture to boost range or the roll.'
          : 'After rolling, you can choose whether to spend culture to reach 5.'}{#if origin?.reroll}
          Buddhism offers one optional reroll.{/if}
      </p>
      {#if cheaper}
        <div class="influence-source-warning" role="status">
          <p>
            <TriangleAlert size={16} />{cheaper.position} can influence this same target with a lower range cost.
          </p>
          <button
            class="secondary wide"
            disabled={$session.pending}
            onclick={() => {
              controller.selectInfluenceOrigin(cheaper.position);
              confirmedSource = null;
            }}
          >
            Switch source to {cheaper.position}<span
              >Save <ResourceAmount
                pile={{
                  culture_tokens: (rangePayment.culture_tokens ?? 0) - (cheaper.payment.culture_tokens ?? 0),
                }}
                compact
              /></span
            >
          </button>
        </div>
      {/if}
      <button class="primary wide" disabled={$session.pending} onclick={roll}>
        {cheaper && confirmedSource === attemptKey
          ? `Use ${origin?.position ?? target.origin} anyway`
          : Object.values(upfront).some(Boolean)
            ? 'Pay & roll'
            : 'Roll for influence'}{#if Object.values(upfront).some(Boolean)}<ResourceAmount
            pile={upfront}
            compact
          />{/if}<ArrowRight size={16} />
      </button>
    </div>
  {/if}
</div>

<style>
  .influence-source-warning {
    margin: 0.8rem 0;
    padding: 0.75rem;
    border: 1px solid var(--gold, #b79351);
    border-radius: 6px;
  }
  .influence-source-warning p {
    display: flex;
    gap: 0.5rem;
    align-items: start;
    margin: 0 0 0.65rem;
    font-size: 0.85rem;
  }
  .influence-source-warning p :global(svg) {
    flex-shrink: 0;
  }
  .influence-source-warning button > span {
    display: inline-flex;
    gap: 0.4rem;
    align-items: center;
  }
</style>
