<script lang="ts">
  import { Smile, Meh, Frown, Landmark, ArrowRight, Check, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Move, Pile } from './types';
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
  <div class="happiness-city-list">
    {#each offers as { city, targets }}
      {@const CurrentMood = city.mood === 'Happy' ? Smile : city.mood === 'Angry' ? Frown : Meh}
      <div class="mood-city-row" class:multiple-targets={targets.length > 1}>
        <div class="mood-city-name">
          <strong>{city.position}</strong><span title={`Size ${city.size}`} aria-label={`Size ${city.size}`}
            ><Landmark size={13} />{city.size}</span
          >
        </div>
        <span
          class="mood-current"
          title={`Currently ${city.mood.toLowerCase()}`}
          aria-label={`Currently ${city.mood.toLowerCase()}`}><CurrentMood size={21} /></span
        >
        {#if targets.length}<ArrowRight size={15} class="mood-change-arrow" />{/if}
        <div class="mood-targets" role="group" aria-label={`Happiness at ${city.position}`}>
          {#each targets as target}{@const Icon = target.mood === 'Happy' ? Smile : Meh}
            <button
              class:selected={steps[city.position] === target.steps}
              class:primary={singleCity && !!target.action}
              aria-label={`${singleCity ? 'Make' : 'Set'} ${city.position} ${target.mood.toLowerCase()}`}
              aria-pressed={singleCity ? undefined : steps[city.position] === target.steps}
              title={target.reason ??
                (singleCity ? 'Improve this city' : 'Cost if improving this city alone')}
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
              <span class="mood-target-label"><Icon size={18} />{target.mood}</span>
              <span class="mood-target-cost"
                ><ResourceAmount
                  pile={Object.values(target.payment).some(Boolean) ? target.payment : { mood_tokens: 0 }}
                  showZero={!Object.values(target.payment).some(Boolean)}
                />
                {#if singleCity}{#if choices[variant]?.free}<span aria-label="Free action" title="Free action"
                      ><Zap size={13} />0</span
                    >{:else}<span aria-label="Costs 1 action"><Zap size={13} />1</span>{/if}{/if}
              </span>
              {#if !singleCity}<span class="mood-selection-mark" aria-hidden="true"
                  >{#if steps[city.position] === target.steps}<Check size={12} />{/if}</span
                >{/if}
            </button>
          {:else}<span class="mood-already-happy">Happy<Check size={14} /></span>{/each}
        </div>
      </div>
    {/each}
  </div>
  {#if !singleCity}
    {#if preview.error}<p class="inline-error">{preview.error}</p>{/if}
    <footer class="happiness-footer">
      <button
        class="primary wide happiness-confirm"
        disabled={!preview.action || $session.pending}
        onclick={() => preview.action && controller.submit(preview.action)}
      >
        <span
          >{selected.length
            ? `Improve ${selected.length} ${selected.length === 1 ? 'city' : 'cities'}`
            : 'Select cities to improve'}</span
        >
        <span class="happiness-total" title="Total cost for all selected cities">
          {#if preview.payment}<ResourceAmount pile={preview.payment} />{/if}
          {#if choices[variant]?.free}<span aria-label="Free action" title="Free action"
              ><Zap size={13} />0</span
            >{:else}<span class="settler-action-cost" aria-label="Costs 1 action"><Zap size={14} />1</span
            >{/if}
        </span>
      </button>
    </footer>
  {/if}
</div>
