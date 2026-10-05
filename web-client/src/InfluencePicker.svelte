<script lang="ts">
  import { Drama, ArrowRight, Landmark, Footprints, RotateCcw, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { influenceUpfrontCost, type InfluenceOffer } from './influence';
  import ResourceAmount from './ResourceAmount.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  const offers = $derived($session.view?.influence ?? []);
  const key = (offer: InfluenceOffer) => `${offer.position}/${offer.name}/${offer.variant}`;
  let chosen = $state<string | null>(null);
  let chosenOrigin = $state<string | null>(null);
  const target = $derived(offers.find((o) => key(o) === chosen));
  const origin = $derived(
    target?.origins?.find((o) => o.position === (chosenOrigin ?? target.origin)) ?? target?.origins?.[0],
  );
  const rangePayment = $derived(origin?.payment ?? target?.payment ?? {});
  const actionPayment = $derived(origin?.actionPayment ?? target?.actionPayment ?? {});
  const upfront = $derived(influenceUpfrontCost(actionPayment, rangePayment));
  const groups = $derived([...new Set(offers.map((o) => o.position))]);
  const bonus = $derived(origin?.rollBonus ?? target?.rollBonus ?? 0);
  const preventBoost = $derived(origin?.preventBoost ?? target?.preventBoost ?? false);
</script>

<div class="influence-picker">
  <h3><Drama size={18} />Cultural influence</h3>
  <ol class="influence-steps" aria-label="Influence progress">
    <li class="current" aria-current="step">Target</li>
    <li>Roll</li>
    <li>Resolve</li>
  </ol>
  {#if !target}
    <p class="movement-hint">
      Choose a target, then the city to influence from. Only one successful attempt per turn.
    </p>
    <div class="influence-targets">
      {#each groups as position}<div class="influence-target-group">
          <strong>At {position}</strong>
          <div class="decision-options">
            {#each offers.filter((o) => o.position === position) as offer}<button
                class:selected={chosen === key(offer)}
                aria-pressed={chosen === key(offer)}
                onmouseenter={() => onHighlight(offer.position)}
                onmouseleave={() => onHighlight(null)}
                onfocus={() => onHighlight(offer.position)}
                onblur={() => onHighlight(null)}
                onclick={() => {
                  chosen = key(offer);
                  chosenOrigin = null;
                }}>{offer.name}<small>{offer.variant}</small></button
              >{/each}
          </div>
        </div>{:else}<p class="settler-empty">No eligible targets.</p>{/each}
    </div>
  {:else}<button
      class="secondary compact influence-change"
      onclick={() => {
        chosen = null;
        chosenOrigin = null;
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
              onclick={() => (chosenOrigin = from.position)}
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
      <button
        class="primary wide"
        disabled={$session.pending}
        onclick={() =>
          controller.startInfluence(origin?.action ?? target.action, actionPayment, rangePayment)}
      >
        {Object.values(upfront).some(Boolean)
          ? 'Pay & roll'
          : 'Roll for influence'}{#if Object.values(upfront).some(Boolean)}<ResourceAmount
            pile={upfront}
            compact
          />{/if}<ArrowRight size={16} />
      </button>
    </div>
  {/if}
</div>
