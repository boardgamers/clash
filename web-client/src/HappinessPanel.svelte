<script lang="ts">
  import { Smile, Meh, Plus, Minus, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Move, Pile } from './types';
  import CityFacts from './CityFacts.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let steps = $state<Record<string, number>>({});
  let variant = $state(0);
  const choices = $derived($session.view?.happinessActions ?? []);
  const selected = $derived(Object.entries(steps).filter(([, n]) => n > 0));
  const preview = $derived.by(() => {
    if (!selected.length || !choices[variant]) return { action: null, payment: null, error: '' };
    try {
      return {
        ...controller.query<{ action: Move; payment: Pile }>({
          kind: 'happiness',
          cities: selected,
          variant: choices[variant].value,
        }),
        error: '',
      };
    } catch (error) {
      return { action: null, payment: null, error: String(error) };
    }
  });
</script>

<div class="happiness-selection">
  {#if choices.length > 1}<div class="variant-picker">
      {#each choices as choice, i}<button
          class:selected={variant === i}
          aria-pressed={variant === i}
          onclick={() => (variant = i)}
          >{choice.name}{#if !choice.free}<Zap size={12} />1{/if}</button
        >{/each}
    </div>{/if}
  {#each $session.view?.cities ?? [] as city}
    {@const max = city.mood === 'Happy' ? 0 : city.mood === 'Neutral' ? 1 : 2}
    <div class="happiness-city">
      <strong>{city.position}</strong><CityFacts size={city.size} mood={city.mood} /><span
        class="happiness-target"
        >{#if steps[city.position]}→ {#if steps[city.position] === max}<Smile size={19} />{:else}<Meh
              size={19}
            />{/if}{/if}</span
      >
      <div class="quantity">
        <button
          aria-label={`Less happiness at ${city.position}`}
          disabled={!steps[city.position] || $session.pending}
          onclick={() => (steps = { ...steps, [city.position]: (steps[city.position] ?? 0) - 1 })}
          ><Minus size={14} /></button
        ><output>{steps[city.position] ?? 0}</output><button
          aria-label={`More happiness at ${city.position}`}
          disabled={!choices.length || (steps[city.position] ?? 0) >= max || $session.pending}
          onclick={() => (steps = { ...steps, [city.position]: (steps[city.position] ?? 0) + 1 })}
          ><Plus size={14} /></button
        >
      </div>
    </div>
  {/each}
  {#if preview.payment}<ResourceAmount pile={preview.payment} />{/if}
  {#if preview.error}<p class="inline-error">{preview.error}</p>{/if}
  <button
    class="primary wide"
    disabled={!preview.action || $session.pending}
    onclick={() => preview.action && controller.submit(preview.action)}
    >Improve happiness{#if !choices[variant]?.free}<span
        class="settler-action-cost"
        aria-label="Costs 1 action"><Zap size={14} />1</span
      >{/if}</button
  >
</div>
