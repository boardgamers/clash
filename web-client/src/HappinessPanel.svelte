<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Smile, Meh, ArrowRight, Check, Zap, X } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { happinessPreview, happinessTargets } from './happiness';
  import ResourceAmount from './ResourceAmount.svelte';
  import CityFacts from './CityFacts.svelte';
  import ContextualCards from './ContextualCards.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  onDestroy(() => onHighlight(null));
  const steps = $derived($session.happinessSteps ?? {});
  const choices = $derived($session.view?.happinessActions ?? []);
  const variant = $derived(choices[$session.happinessVariant ?? 0]);
  const cities = $derived($session.view?.cities ?? []);
  const selected = $derived(cities.filter((city) => steps[city.position] > 0));
  const focused = $derived(cities.find((city) => city.position === $session.happinessCity));
  const targets = $derived(
    focused ? happinessTargets($session, (input) => controller.query(input), focused.position) : [],
  );
  const preview = $derived(happinessPreview($session, (input) => controller.query(input)));
  const cityName = (position: string) =>
    cities.find((c) => c.position === position)?.capital
      ? 'Capital'
      : `City ${cities.findIndex((c) => c.position === position) + 1}`;
  const targetMood = (position: string) =>
    cities.find((c) => c.position === position)?.mood === 'Angry' && steps[position] === 1
      ? 'Neutral'
      : 'Happy';
</script>

<section class="action-panel floating-panel selection-tray happiness-panel" aria-label="Increase happiness">
  <header class="movement-heading">
    <h2><Smile size={18} />Happiness</h2>
    <div class="happiness-budget" title="Mood tokens available">
      <ResourceAmount
        pile={{
          mood_tokens:
            $session.game?.players.find((p) => p.id === $session.seat)?.resources?.mood_tokens ?? 0,
        }}
        showZero
      />
    </div>
    <button
      class="icon-button close-movement"
      aria-label="Close happiness controls"
      onclick={() => controller.patch({ mode: 'overview', error: '' })}><X size={18} /></button
    >
  </header>
  <ContextualCards {controller} context="happiness" />
  {#if choices.length > 1}<div class="variant-picker" aria-label="Happiness action">
      {#each choices as choice, i}<button
          class:selected={($session.happinessVariant ?? 0) === i}
          aria-pressed={($session.happinessVariant ?? 0) === i}
          disabled={$session.pending}
          onclick={() => controller.switchHappinessVariant(i)}
        >
          {choice.name}<span
            class="happiness-action-cost"
            aria-label={choice.free ? 'Free action' : 'Costs 1 action'}
            ><Zap size={12} />{choice.free ? 0 : 1}</span
          >
        </button>{/each}
    </div>{/if}
  <p class="movement-hint">Select cities on the map. Tap again to remove.</p>
  {#if selected.length}<div class="happiness-selected" aria-label="Selected cities">
      {#each selected as city}<button
          class:active={focused?.position === city.position}
          aria-label={`Adjust happiness at ${city.position}`}
          title={`Adjust ${cityName(city.position)}`}
          onmouseenter={() => onHighlight(city.position)}
          onmouseleave={() => onHighlight(null)}
          onfocus={() => onHighlight(city.position)}
          onblur={() => onHighlight(null)}
          onclick={() => controller.patch({ happinessCity: city.position, focus: city.position })}
        >
          {cityName(city.position)}<CityFacts size={city.size} mood={targetMood(city.position)} />
        </button>{/each}
    </div>{/if}
  {#if focused}
    <div class="happiness-target-row">
      <span class="happiness-city-name"
        >{cityName(focused.position)}<CityFacts size={focused.size} mood={focused.mood} /><ArrowRight
          size={14}
        /></span
      >
      <div class="happiness-targets" role="group" aria-label={`Happiness at ${focused.position}`}>
        {#each targets as target}{@const isSelected =
            steps[focused.position] === target.steps &&
            !!target.lawgiver === ($session.happinessLawgiver === focused.position)}{@const Icon =
            target.mood === 'Happy' ? Smile : Meh}
          <button
            class:selected={isSelected}
            aria-pressed={isSelected}
            disabled={!target.action || $session.pending}
            title={target.reason ??
              (target.lawgiver
                ? 'Lawgiver: make Hammurabi’s city happy for 1 culture token'
                : `Set ${cityName(focused.position)} ${target.mood.toLowerCase()}`)}
            onclick={() => controller.setHappinessCity(focused.position, target.steps, target.lawgiver)}
          >
            <span class="city-mood" data-mood={target.mood.toLowerCase()}><Icon size={18} /></span
            >{target.lawgiver ? 'Lawgiver' : target.mood}
          </button>
        {/each}
        {#if steps[focused.position]}<button
            class="icon-button"
            aria-label={`Remove ${cityName(focused.position)} from happiness selection`}
            title="Remove city"
            disabled={$session.pending}
            onclick={() => controller.setHappinessCity(focused.position, 0)}><X size={15} /></button
          >{/if}
      </div>
    </div>
    {#if !steps[focused.position] && !targets.some((target) => target.action)}<p class="inline-error">
        {targets[0]?.reason ?? 'This city cannot be improved now.'}
      </p>{/if}
  {/if}
  {#if !cities.some((city) => city.mood !== 'Happy')}<p class="movement-hint">
      All your cities are already happy.
    </p>{/if}
  {#if preview.reason}<p class="inline-error">{preview.reason}</p>{/if}
  <footer class="happiness-map-footer">
    <span class="happiness-total" aria-label="Total happiness cost">
      {#if selected.length}<ResourceAmount pile={preview.payment} />{/if}
      <span title={variant?.free ? 'Free action' : 'Costs 1 action'}
        ><Zap size={13} />{variant?.free ? 0 : 1}</span
      >
    </span>
    <button
      class="primary"
      disabled={!preview.action || $session.pending}
      onclick={() => preview.action && controller.submit(preview.action)}
    >
      {selected.length
        ? `Improve ${selected.length} ${selected.length === 1 ? 'city' : 'cities'}`
        : 'Improve cities'}<Check size={15} />
    </button>
  </footer>
</section>
