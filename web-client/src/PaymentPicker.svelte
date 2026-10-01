<script lang="ts">
  import { Check, Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link } from 'lucide-svelte';
  import { resources, resourceNames, type Pile, type Resource } from './types';
  import { samePayment, paymentWithAmount } from './payment-options';
  import { pileText } from './model';
  import ResourceAmount from './ResourceAmount.svelte';
  let {
    options,
    value,
    onChange,
    onPay,
    pending = false,
    optional = false,
    reward = false,
    label = 'Payment',
  }: {
    options: { payment: Pile; disabled?: boolean }[];
    value: Pile;
    onChange: (payment: Pile) => void;
    onPay?: (payment: Pile) => void;
    pending?: boolean;
    optional?: boolean;
    reward?: boolean;
    label?: string;
  } = $props();
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
    captives: Link,
  };
  const empty = (payment: Pile) => !Object.values(payment).some(Boolean);
  const available = $derived(options.filter((option) => !option.disabled));
  const paid = $derived(available.filter((option) => !empty(option.payment)));
  const selected = $derived(available.find((option) => samePayment(option.payment, value)) ?? available[0]);
  const emptyOption = $derived(options.find((option) => empty(option.payment)));
  const fields = $derived(
    resources
      .map((resource) => ({
        resource,
        amounts: [...new Set(paid.map((option) => option.payment[resource] ?? 0))].sort((a, b) => a - b),
      }))
      .filter((field) => field.amounts.some(Boolean)),
  );
  const fixed = $derived(
    Object.fromEntries(
      fields.filter((field) => field.amounts.length === 1).map((field) => [field.resource, field.amounts[0]]),
    ) as Pile,
  );
  const long = $derived(options.some((option) => Object.values(option.payment).filter(Boolean).length > 3));
  const emptyLabel = $derived(optional ? 'Decline' : onPay ? 'Continue' : 'Free');
  const verb = $derived(reward ? 'Gain' : 'Pay');
  function optionLabel(payment: Pile) {
    return empty(payment) ? emptyLabel : `${verb} ${pileText(payment)}`;
  }
  function choose(payment: Pile) {
    onChange(payment);
  }
  function setAmount(resource: Resource, amount: number) {
    if (selected)
      choose(
        paymentWithAmount(
          paid.map((option) => option.payment),
          selected.payment,
          resource,
          amount,
        ),
      );
  }
  $effect(() => {
    if (selected && !samePayment(selected.payment, value)) onChange(selected.payment);
  });
</script>

{#if options.length <= 4}
  <div
    class="payment-choices compact-payments"
    class:single={options.length === 1}
    class:long-payments={long}
    role="group"
    aria-label={label}
  >
    {#each options as option}
      <button
        class="secondary"
        class:selected={!onPay && samePayment(option.payment, value)}
        aria-label={optionLabel(option.payment)}
        aria-pressed={onPay ? undefined : samePayment(option.payment, value)}
        disabled={pending || option.disabled || (!onPay && options.length === 1)}
        onclick={() => (onPay ? onPay(option.payment) : choose(option.payment))}
      >
        {#if empty(option.payment)}{emptyLabel}{:else}
          {#if onPay}<span>{verb}</span>{/if}<ResourceAmount
            pile={option.payment}
            compact={Object.values(option.payment).filter(Boolean).length > 1}
          />
        {/if}
        {#if !onPay && samePayment(option.payment, value)}<Check size={14} />{/if}
      </button>
    {:else}<p class="inline-error">No affordable payment available.</p>{/each}
  </div>
{:else}
  <div class="payment-builder" role="group" aria-label={label}>
    {#if selected && !empty(selected.payment)}
      {#if Object.keys(fixed).length}<div class="payment-fixed">
          <span>{reward ? 'Included' : 'Fixed cost'}</span><ResourceAmount pile={fixed} />
        </div>{/if}
      <div class="payment-builder-amounts">
        {#each fields.filter((field) => field.amounts.length > 1) as field}
          {@const Icon = icons[field.resource]}
          <label
            ><span><Icon size={15} />{resourceNames[field.resource]}</span>
            <select
              aria-label={`${resourceNames[field.resource]} amount for ${label}`}
              disabled={pending}
              value={selected.payment[field.resource] ?? 0}
              onchange={(event) => setAmount(field.resource, Number(event.currentTarget.value))}
            >
              {#each field.amounts as amount}<option value={amount}>{amount}</option>{/each}
            </select>
          </label>
        {/each}
      </div>
      <small class="payment-builder-hint">Changing one amount adjusts the others.</small>
      <div class="payment-builder-total">
        <span>{reward ? 'You gain' : 'You pay'}</span><ResourceAmount pile={selected.payment} />
      </div>
      <div class="payment-builder-actions">
        {#if emptyOption}<button
            class="secondary"
            disabled={pending || emptyOption.disabled}
            onclick={() => (onPay ? onPay(emptyOption.payment) : choose(emptyOption.payment))}
            >{emptyLabel}</button
          >{/if}
        {#if onPay}<button
            class="primary"
            aria-label={optionLabel(selected.payment)}
            disabled={pending}
            onclick={() => selected && onPay?.(selected.payment)}>{verb}<Check size={15} /></button
          >{/if}
      </div>
    {:else if selected}
      <div class="payment-builder-actions">
        {#if paid.length}<button class="secondary" disabled={pending} onclick={() => choose(paid[0].payment)}
            >Choose payment</button
          >{/if}
        {#if onPay}<button
            class="primary"
            disabled={pending}
            onclick={() => selected && onPay?.(selected.payment)}>{emptyLabel}</button
          >{:else}<span>{optional ? 'Declined' : 'Free'}</span>{/if}
      </div>
    {:else}<p class="inline-error">No affordable payment available.</p>{/if}
  </div>
{/if}
