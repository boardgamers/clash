<script lang="ts">
  import {
    Drama,
    Zap,
    ArrowRight,
    ArrowLeft,
    Landmark,
    Footprints,
    RotateCcw,
    X,
    TriangleAlert,
  } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { activeInfluence, influenceKey, influenceUpfrontCost, cheaperInfluenceOrigin, influenceDestinations, influenceDestinationKey, type InfluenceOffer } from './influence';
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
  const methods = $derived(target ? offers.filter(o => influenceDestinationKey(o) === influenceDestinationKey(target) && (!origin || o.origins?.some(from => from.position === origin.position))) : []);
  const paymentStep = $derived(!!$session.influencePaymentStep);
  const selectionKey = $derived(`${target ? influenceDestinationKey(target) : ''}:${origin?.position}`);
  $effect(() => { selectionKey; controller.patch({ influencePaymentStep: false }); });
  const cheaper = $derived(cheaperInfluenceOrigin(target, origin));
  const attemptKey = $derived(`${target ? influenceKey(target) : ''}:${origin?.position}`);
  let confirmedSource = $state<string | null>(null);
  $effect(() => {
    attemptKey;
    confirmedSource = null;
  });
  function roll(method: InfluenceOffer) {
    const from = method.origins?.find(o => o.position === origin?.position);
    if (cheaper && confirmedSource !== attemptKey) {
      confirmedSource = attemptKey;
      return;
    }
    controller.startInfluence(from?.action ?? method.action, from?.actionPayment ?? method.actionPayment ?? {}, from?.payment ?? method.payment);
  }
  const culture = $derived($session.game?.players[$session.seat ?? 0]?.resources?.culture_tokens ?? 0);
  const rangePayment = $derived(origin?.payment ?? target?.payment ?? {});
  // A city clicked on the map narrows the list to its own candidates.
  const groups = $derived(influence?.position ? [influence.position] : []);
  const bonus = $derived(origin?.rollBonus ?? target?.rollBonus ?? 0);
  const preventBoost = $derived(origin?.preventBoost ?? target?.preventBoost ?? false);
</script>

<header class="movement-heading">
  <button
    class="icon-button"
    aria-label="Previous influence step"
    onclick={() => controller.backInfluence()}><ArrowLeft size={17} /></button
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
    {#each ['Hex', 'Destination', 'Source', 'Payment'] as label, i}
      <li class:current={i === (!influence?.position ? 0 : !target ? 1 : paymentStep ? 3 : 2)} aria-current={i === (!influence?.position ? 0 : !target ? 1 : paymentStep ? 3 : 2) ? 'step' : undefined}>{label}</li>
    {/each}
  </ol>
  {#if !target}
    <p class="movement-hint">{influence?.position ? 'Choose a building in this hex, on the map or below.' : 'Choose a highlighted hex on the map.'}</p>
    {#if influence?.position}<div class="influence-targets">
      {#each groups as position}<div class="influence-target-group">
          <strong>At {position}</strong>
          <div class="decision-options">
            {#each influenceDestinations(offers.filter((o) => o.position === position)) as offer}<button
                onmouseenter={() => onHighlight(offer.position)}
                onmouseleave={() => onHighlight(null)}
                onfocus={() => onHighlight(offer.position)}
                onblur={() => onHighlight(null)}
                onclick={() => {
                  controller.selectInfluenceTarget(influenceKey(offer));
                  onHighlight(null);
                }}>{offer.name}</button
              >{/each}
          </div>
        </div>{:else}<p class="settler-empty">No eligible targets.</p>{/each}
    </div>{/if}
  {/if}
  {#if !target && influence?.position}<button
      class="secondary compact influence-change"
      onclick={() => {
        controller.patch({ influencePosition: null, influenceTarget: null, influenceOrigin: null });
        onHighlight(null);
      }}>Change target</button
    >{/if}
  {#if target}
    <div class="influence-preview">
      <div class="influence-route">
        <button class="route-choice" aria-label="Change source" onclick={() => controller.patch({ influencePaymentStep: false })}>
          <small>From</small><strong>{origin?.position ?? target.origin}</strong>
        </button>
        <ArrowRight size={16} />
        <button class="route-choice route-target" aria-label="Change target" onclick={() => {
          controller.patch({ influencePosition: null, influenceTarget: null, influenceOrigin: null });
          onHighlight(null);
        }}><small>Target</small><strong>{target.name} at {target.position}</strong></button>
      </div>
      {#if !paymentStep && (target.origins?.length ?? 0) > 0}<div class="influence-origins" aria-label="Influence from">
          <span>From</span>{#each target.origins ?? [] as from}<button
              class:selected={origin === from}
              aria-pressed={origin === from}
              title={from.position === target.position ? 'Same hex: cannot boost the roll' : from.reroll ? 'Buddhism reroll available' : from.settlers ? 'Settlers' : 'City'}
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
              {#if from.position === target.position}<span class="source-boost-warning" aria-label="Same hex: cannot boost the roll"><TriangleAlert size={15} /></span>{/if}
              {#if from.reroll}<RotateCcw size={14} />{/if}
            </button>{/each}
        </div>{/if}
      {#if !paymentStep}
        <p class="movement-hint">Choose a source on the map or above. When reclaiming, another city is preferred so you can boost the roll.</p>
        <button class="primary wide" onclick={() => controller.patch({ influencePaymentStep: true })}>Choose payment<ArrowRight size={16} /></button>
      {:else}
      {#if Object.values(rangePayment).some(Boolean)}
        <p class="range-note">Includes <ResourceAmount pile={rangePayment} compact /> for extra range.</p>
      {/if}

      {#if cheaper}
        <div class="influence-source-warning" role="status">
          <p>
            <TriangleAlert size={16} />{cheaper.position} can influence this same target with a lower range cost.
            {#if confirmedSource === attemptKey}Choose Influence or Arts again to keep the current source.{/if}
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

      {/if}
    </div>
  {/if}
</div>

{#if target && paymentStep}
<div class="influence-submit">
      <div class="roll-summary">
        <strong>Roll {Math.max(1, 5 - bonus)}+ to succeed</strong>
        <span>{origin?.position === target?.position
          ? 'Self-targeting · No roll boost'
          : preventBoost
          ? 'Protected target · No range or roll boost'
          : 'Boost after rolling · 1 culture per +1.'}</span>
        {#if origin?.reroll}<span>Buddhism · One optional reroll</span>{/if}
      </div>
      <div class="influence-methods" aria-label="Influence payment method">
        {#each methods as method}
          {@const from = method.origins?.find(o => o.position === origin?.position)}
          {@const cost = influenceUpfrontCost(from?.actionPayment ?? method.actionPayment, from?.payment ?? method.payment)}
          {@const maxBoost = origin?.position === target.position || (from?.preventBoost ?? method.preventBoost) ? 0 : Math.min(Math.max(0, 4 - (from?.rollBonus ?? method.rollBonus ?? 0)), Math.max(0, culture - (cost.culture_tokens ?? 0)))}
          <button class="primary" disabled={$session.pending} onclick={() => roll(method)}>
            <span class="method-title">{method.variant === 'Arts' ? 'Arts' : 'Influence'}
              <ResourceAmount pile={cost} compact /><span class="influence-action-cost" aria-label={(from?.free ?? method.free) ? 'Free action' : 'Costs 1 action'} title={(from?.free ?? method.free) ? 'Free action' : 'Costs 1 action'}>{(from?.free ?? method.free) ? 0 : 1}<Zap size={13} /></span>
            </span>
            <small>Maximum roll boost: +{maxBoost}</small>
          </button>
        {/each}
      </div>

</div>
{/if}

<style>
  .source-boost-warning { display: inline-flex; color: var(--gold, #b79351); }
  .influence-picker { min-height: 0; overflow-y: auto; }
  .influence-submit { flex-shrink: 0; padding-top: 0.4rem; border-top: 1px solid var(--line); }
  .movement-heading { flex-shrink: 0; }

  .influence-route { display: flex; flex-wrap: nowrap; gap: 0.75rem; margin-bottom: 0.65rem; }
  .route-choice { display: flex; flex-direction: column; align-items: flex-start; gap: 0.2rem; padding: 0.6rem 0.8rem; text-align: left; }
  .route-choice small { color: var(--muted); font-size: 11px; }
  .route-choice strong { font-size: 14px; }
  .route-target { flex: 1; min-width: 0; }
  .influence-methods { display: flex; gap: 0.5rem; margin: 0; }
  .influence-methods button { flex: 1; flex-direction: column; align-items: stretch; gap: 0.3rem; }
  .influence-action-cost { display: inline-flex; align-items: center; gap: 0.2rem; color: inherit; font-size: inherit; line-height: 1; }
  .method-title { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .influence-methods small { font-size: 11px; color: inherit; }
  @media (min-width: 700px) {
    .influence-methods { flex-direction: column; }
    .influence-methods button { flex-direction: row; align-items: center; justify-content: space-between; flex-wrap: wrap; }
    .method-title { flex-wrap: nowrap; }
    .influence-methods button > small { white-space: nowrap; }
  }
  .roll-summary { display: grid; gap: 0.3rem; margin: 0.2rem 0 0.6rem; font-size: 12px; color: var(--muted); line-height: 1.5; }
  .roll-summary strong { color: var(--ink); font-size: 14px; }
  .range-note { display: flex; align-items: center; gap: 0.35rem; font-size: 12px; color: var(--muted); }

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
