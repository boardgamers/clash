<script lang="ts">
  import { onDestroy } from 'svelte';
  import { untrack } from 'svelte';
  import { Check, Sparkles, Layers, BookOpen, Ship, ChevronRight, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Decision, Move, Pile, Resource } from './types';
  import { resourceNames } from './types';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import DecisionOptionContent from './DecisionOptionContent.svelte';
  import { researchPresentation } from './research';
  import { mapDecisionOptions } from './decision-controls';
  import { pileText } from './model';
  let {
    controller,
    decision,
    onHighlight,
  }: { controller: Controller; decision: Decision; onHighlight: (position: string | null) => void } =
    $props();
  const session = $derived(controller.session);
  const panelId = $props.id();
  const selected = $derived($session.decisionSelection);
  const mapChoice = $derived(mapDecisionOptions(decision).length > 0);
  let payments = $state<Pile[]>(
    untrack(() => decision.fields.map((f) => ({ ...(f.choices?.length === 1 ? f.choices[0] : f.initial) }))),
  );
  const samePayment = (a: Pile, b: Pile) =>
    Object.keys(resourceNames).every((r) => (a[r as Resource] ?? 0) === (b[r as Resource] ?? 0));
  const emptyPayment = (p: Pile) => !Object.values(p).some(Boolean);
  const directPayments = $derived.by(() => {
    const field = decision.fields.length === 1 && !decision.options.length ? decision.fields[0] : undefined;
    if (!field?.choices) return null;
    return field.choices.map((payment) => {
      try {
        return {
          payment,
          ...controller.query<{ action: Move }>({ kind: 'decision', values: [], payments: [payment] }),
        };
      } catch {
        return { payment, action: null };
      }
    });
  });
  function paymentLabel(payment: Pile, optional: boolean) {
    return emptyPayment(payment)
      ? optional
        ? 'Decline'
        : 'Continue'
      : `${decision.reward ? 'Gain' : 'Pay'} ${pileText(payment)}`;
  }
  function choosePayment(i: number, payment: Pile) {
    payments = payments.map((p, j) => (i === j ? payment : p));
  }
  const preview = $derived.by(() => {
    try {
      return {
        ...controller.query<{ action: Move }>({
          kind: 'decision',
          values: selected.map((i) => decision.options[i].value),
          payments,
        }),
        error: '',
      };
    } catch (error) {
      return { action: null, error: String(error) };
    }
  });
  const count = $derived(
    decision.min === decision.max ? `Choose ${decision.max}` : `Choose ${decision.min}–${decision.max}`,
  );
  function toggle(i: number) {
    controller.selectDecisionOption(i);
  }
  onDestroy(() => onHighlight(null));
</script>

{#snippet options()}
  <div class="decision-options">
    {#each decision.options as option, i}
      {@const advance =
        typeof option.value === 'string'
          ? $session.view?.advances.find((a) => a.id === option.value)
          : undefined}
      {@const Icon = advance ? researchPresentation(advance).icon : null}
      <button
        class="decision-option"
        class:has-description={!!option.description}
        class:card-option={!!option.card}
        class:selected={selected.includes(i)}
        aria-label={option.name}
        aria-describedby={option.description ? `${panelId}-option-${i}` : undefined}
        aria-pressed={selected.includes(i)}
        disabled={$session.pending ||
          (!selected.includes(i) && decision.max > 1 && selected.length >= decision.max)}
        onmouseenter={() => onHighlight(option.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(option.position)}
        onblur={() => onHighlight(null)}
        onclick={() => toggle(i)}
      >
        {#if option.terrain}<TerrainIcon terrain={option.terrain} />{:else if Icon}<Icon size={19} />{/if}
        <DecisionOptionContent {option} id={`${panelId}-option-${i}`} />
        <span class="decision-selection" aria-hidden="true"
          >{#if selected.includes(i)}<Check size={13} />{/if}</span
        >
      </button>
    {/each}
  </div>
{/snippet}

<section
  class="action-panel floating-panel decision-panel"
  class:board-decision={mapChoice}
  class:card-decision={decision.options.some((option) => option.card)}
  aria-label={decision.name}
>
  {#if decision.endOfAge}<span class="tiny-label">END OF AGE</span>{/if}
  <h2><Sparkles size={21} />{decision.name}</h2>
  {#if decision.eventContext}
    {@const context = decision.eventContext}
    <div class="decision-event-context">
      {#if context.raid}<p class="event-raid"><Ship size={17} /><ResourceText text={context.raid} /></p>{/if}
      {#if context.card}
        {@const card = context.card}
        <div class="event-card-offer">
          <Layers size={16} />
          <span
            >{card.later
              ? card.firstOffer
                ? 'Then: take the card'
                : 'If passed to you'
              : 'Take the card'}</span
          >
          <ResourceAmount pile={card.cost} />
        </div>
        <details class="event-rules">
          <summary aria-label={`${card.name} event details`}
            ><BookOpen size={13} />{card.name}<ChevronRight size={13} /></summary
          >
          {#if context.placement}<p>{context.placement}</p>{/if}
          <p>
            The player who triggered the event may take the card for <ResourceAmount
              pile={{ culture_tokens: 1 }}
            />. If they pass, other players may take it in turn order for <ResourceAmount
              pile={{ culture_tokens: 2 }}
            />.
          </p>
          <p>
            <span class="event-card-use">Play <Zap size={13} />{card.free ? 0 : 1}</span><ResourceText
              text={card.description}
            />
          </p>
        </details>
      {:else}
        <details class="event-rules">
          <summary aria-label={`${context.name} event details`}
            ><BookOpen size={13} />Event rules<ChevronRight size={13} /></summary
          >
          {#if context.placement}<p>{context.placement}</p>{/if}
          {#each context.rules as rule}<p><ResourceText text={rule} /></p>{/each}
        </details>
      {/if}
    </div>
  {/if}
  {#if decision.description}<p class="decision-description">
      <ResourceText
        text={decision.description
          .replaceAll('Status Phase', 'End of age')
          .replaceAll('status phase', 'end of age')}
      />
    </p>{/if}
  {#if decision.options.length}
    {#if !mapChoice}<div class="decision-count">
        {count}<span>{selected.length}/{decision.max}</span>
      </div>{/if}
    {#if mapChoice}
      <details class="decision-tile-list">
        <summary>Select on map · {selected.length}/{decision.max}<span>Tile list</span></summary>
        {@render options()}
      </details>
    {:else}
      {@render options()}
    {/if}
  {/if}
  {#if directPayments}
    {@const field = decision.fields[0]}
    {#if field.name !== decision.name}<p class="decision-description">
        <ResourceText text={field.name} />
      </p>{/if}
    <div class="payment-choices" class:single={directPayments.length === 1}>
      {#each directPayments as choice}
        <button
          class="secondary"
          aria-label={paymentLabel(choice.payment, field.optional)}
          disabled={$session.pending || !choice.action}
          onclick={() => choice.action && controller.submit(choice.action)}
        >
          {#if emptyPayment(choice.payment)}{field.optional ? 'Decline' : 'Continue'}{:else}
            {decision.reward ? 'Gain' : 'Pay'}
            <ResourceAmount pile={choice.payment} compact={Object.keys(choice.payment).length > 1} />
          {/if}
        </button>
      {:else}<p class="inline-error">No affordable payment available.</p>{/each}
    </div>
  {:else}
    {#each decision.fields as field, i}
      <div class="decision-payment">
        {#if decision.fields.length > 1 || field.name !== decision.name}<strong
            ><ResourceText text={field.name} /></strong
          >{/if}
        {#if field.choices}
          <div
            class="payment-choices"
            class:single={field.choices.length === 1}
            role="group"
            aria-label={field.name}
          >
            {#each field.choices as payment}
              <button
                class="secondary"
                class:selected={samePayment(payment, payments[i])}
                aria-pressed={samePayment(payment, payments[i])}
                aria-label={paymentLabel(payment, field.optional)}
                disabled={$session.pending || field.choices.length === 1}
                onclick={() => choosePayment(i, payment)}
              >
                {#if emptyPayment(payment)}{field.optional ? 'Decline' : 'Free'}{:else}<ResourceAmount
                    pile={payment}
                  />{/if}
                {#if samePayment(payment, payments[i])}<Check size={14} />{/if}
              </button>
            {/each}
          </div>
        {:else}
          {#if !decision.reward}<div class="decision-cost">
              Cost <ResourceAmount pile={field.cost} />
            </div>{/if}
          <div class="payment-amounts">
            {#each field.resources as resource}
              {@const available = decision.reward
                ? 255
                : ($session.game?.players.find((p) => p.id === $session.seat)?.resources?.[resource] ?? 0)}
              <label
                ><span>{resourceNames[resource]}</span>
                <select
                  aria-label={`${resourceNames[resource]} amount for ${field.name}`}
                  disabled={$session.pending}
                  value={payments[i][resource] ?? 0}
                  onchange={(event) =>
                    choosePayment(i, { ...payments[i], [resource]: Number(event.currentTarget.value) })}
                >
                  {#each Array.from({ length: available + 1 }, (_, n) => n) as n}<option value={n}>{n}</option
                    >{/each}
                </select>
              </label>
            {/each}
          </div>
          {#if field.optional}<button
              class="secondary compact"
              disabled={$session.pending}
              onclick={() => choosePayment(i, {})}>Decline</button
            >{/if}
        {/if}
      </div>
    {/each}
    <div class="decision-footer">
      <button
        class="primary wide"
        disabled={!preview.action || $session.pending}
        title={preview.error || 'Confirm selection'}
        onclick={() => preview.action && controller.submit(preview.action)}
      >
        {selected.length === 0 && !decision.fields.length && decision.min === 0
          ? 'Skip'
          : mapChoice && selected.length
            ? `Confirm ${selected.map((i) => decision.options[i].position).join(', ')}`
            : 'Confirm'}<Check size={16} />
      </button>
      {#if preview.error && (selected.length > 0 || decision.fields.length)}<p class="inline-error">
          {preview.error}
        </p>{/if}
    </div>
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
