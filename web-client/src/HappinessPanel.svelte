<script lang="ts">
  import { Smile, Meh, Check, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Move, Pile } from './types';
  import CityFacts from './CityFacts.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let steps = $state<Record<string, number>>({});
  let variant = $state(0);
  const choices = $derived($session.view?.happinessActions ?? []);
  const cities = $derived($session.view?.cities ?? []);
  const singleCity = $derived(cities.length === 1);
  const offers = $derived(
    cities.map((city) => ({
      city,
      targets: ($session.view?.cityActions.find((c) => c.position === city.position)?.happiness ?? []).map(
        (target) => {
          if (!choices[variant]) return { ...target, action: null };
          try {
            return {
              ...target,
              ...controller.query<{ action: Move; payment: Pile }>({
                kind: 'happiness',
                cities: [[city.position, target.steps]],
                variant: choices[variant].value,
              }),
              reason: null,
            };
          } catch (error) {
            return { ...target, action: null, reason: String(error) };
          }
        },
      ),
    })),
  );
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
          onclick={() => {
            variant = i;
            steps = {};
          }}
          >{choice.name}{#if !choice.free}<Zap size={12} />1{/if}</button
        >{/each}
    </div>{/if}
  {#if !singleCity}<p class="happiness-instruction">Choose which cities to improve together.</p>{/if}
  {#each offers as { city, targets }}
    <div class="happiness-city mood-choice-row">
      <div class="mood-city-name">
        <strong>{city.position}</strong><CityFacts size={city.size} mood={city.mood} />
      </div>
      <div class="mood-targets" role="group" aria-label={`Happiness at ${city.position}`}>
        {#each targets as target}{@const Icon = target.mood === 'Happy' ? Smile : Meh}
          <button
            class:selected={steps[city.position] === target.steps}
            class:primary={singleCity && !!target.action}
            aria-label={`${singleCity ? 'Make' : 'Set'} ${city.position} ${target.mood.toLowerCase()}`}
            aria-pressed={singleCity ? undefined : steps[city.position] === target.steps}
            title={target.reason ?? (singleCity ? 'Improve this city' : 'Cost if improving this city alone')}
            disabled={!target.action || $session.pending}
            onclick={() => {
              if (singleCity && target.action) controller.submit(target.action);
              else
                steps = {
                  ...steps,
                  [city.position]: steps[city.position] === target.steps ? 0 : target.steps,
                };
            }}
          >
            <span><Icon size={19} />{singleCity ? `Make ${target.mood.toLowerCase()}` : target.mood}</span>
            <span class="mood-target-cost"
              ><ResourceAmount pile={target.payment} />
              {#if singleCity}{#if choices[variant]?.free}<small>Free action</small>{:else}<span
                    aria-label="Costs 1 action"><Zap size={13} />1</span
                  >{/if}{/if}
            </span>
            {#if !singleCity && steps[city.position] === target.steps}<Check size={15} />{/if}
          </button>
        {:else}<span class="mood-already-happy"><Smile size={17} />Happy</span>{/each}
      </div>
    </div>
  {/each}
  {#if !singleCity}
    {#if preview.error}<p class="inline-error">{preview.error}</p>{/if}
    <button
      class="primary wide happiness-confirm"
      disabled={!preview.action || $session.pending}
      onclick={() => preview.action && controller.submit(preview.action)}
    >
      <span>Improve {selected.length || ''} {selected.length === 1 ? 'city' : 'cities'}</span>
      {#if preview.payment}<ResourceAmount pile={preview.payment} />{/if}
      {#if choices[variant]?.free}<small>Free action</small>{:else}<span
          class="settler-action-cost"
          aria-label="Costs 1 action"><Zap size={14} />1</span
        >{/if}
    </button>
  {/if}
</div>
