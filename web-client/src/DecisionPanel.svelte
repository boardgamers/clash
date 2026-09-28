<script lang="ts">
  import { onDestroy } from 'svelte';
  import { untrack } from 'svelte';
  import { Check, Plus, Minus, CircleHelp, Sparkles } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Decision, Move, Pile, Resource } from './types';
  import { resourceNames } from './types';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import { researchPresentation } from './research';
  let {
    controller,
    decision,
    onHighlight,
  }: { controller: Controller; decision: Decision; onHighlight: (position: string | null) => void } =
    $props();
  const session = $derived(controller.session);
  let selected = $state<number[]>([]);
  let payments = $state<Pile[]>(untrack(() => decision.fields.map((f) => ({ ...f.initial }))));
  let expanded = $state<number | null>(null);
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
    selected = selected.includes(i)
      ? selected.filter((v) => v !== i)
      : decision.max === 1
        ? [i]
        : [...selected, i];
  }
  function adjust(i: number, r: Resource, n: number) {
    payments = payments.map((p, j) =>
      i === j ? { ...p, [r]: Math.max(0, Math.min(255, (p[r] ?? 0) + n)) } : p,
    );
  }
  onDestroy(() => onHighlight(null));
</script>

<section class="action-panel floating-panel decision-panel" aria-label={decision.name}>
  {#if decision.endOfAge}<span class="tiny-label">END OF AGE</span>{/if}
  <h2><Sparkles size={21} />{decision.name}</h2>
  {#if decision.description}<p class="decision-description">
      <ResourceText
        text={decision.description
          .replaceAll('Status Phase', 'End of age')
          .replaceAll('status phase', 'end of age')}
      />
    </p>{/if}
  {#if decision.options.length}
    <div class="decision-count">{count}<span>{selected.length}/{decision.max}</span></div>
    <div class="decision-options">
      {#each decision.options as option, i}
        {@const advance =
          typeof option.value === 'string'
            ? $session.view?.advances.find((a) => a.id === option.value)
            : undefined}
        {@const Icon = advance ? researchPresentation(advance).icon : null}
        <div class="decision-option">
          <button
            class:selected={selected.includes(i)}
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
            <span>{option.name}</span>{#if selected.includes(i)}<Check size={16} />{/if}
          </button>
          {#if option.description}<button
              class="icon-button"
              aria-label={`About ${option.name}`}
              title={advance ? researchPresentation(advance).summary : option.description}
              aria-expanded={expanded === i}
              onclick={() => (expanded = expanded === i ? null : i)}><CircleHelp size={16} /></button
            >{/if}
          {#if expanded === i}<p class="decision-detail"><ResourceText text={option.description} /></p>{/if}
        </div>
      {/each}
    </div>
  {/if}
  {#each decision.fields as field, i}
    <div class="decision-payment">
      {#if decision.fields.length > 1 || field.name !== decision.name}<strong
          ><ResourceText text={field.name} /></strong
        >{/if}
      {#if !decision.reward}<div class="decision-cost"><ResourceAmount pile={field.cost} /></div>{/if}
      <div class="resource-steppers">
        {#each field.resources as resource}
          <div class="resource-stepper">
            <span title={resourceNames[resource]}><ResourceAmount pile={{ [resource]: 1 }} /></span>
            <button
              class="icon-button"
              aria-label={`Less ${resourceNames[resource]} for ${field.name}`}
              disabled={$session.pending || !(payments[i][resource] ?? 0)}
              onclick={() => adjust(i, resource, -1)}><Minus size={13} /></button
            >
            <output>{payments[i][resource] ?? 0}</output>
            <button
              class="icon-button"
              aria-label={`More ${resourceNames[resource]} for ${field.name}`}
              disabled={$session.pending ||
                (payments[i][resource] ?? 0) >=
                  (decision.reward
                    ? 255
                    : ($session.game?.players.find((p) => p.id === $session.seat)?.resources?.[resource] ??
                      0))}
              onclick={() => adjust(i, resource, 1)}><Plus size={13} /></button
            >
          </div>
        {/each}
      </div>
      {#if field.optional}<button
          class="secondary compact"
          disabled={$session.pending}
          onclick={() => (payments = payments.map((p, j) => (i === j ? {} : p)))}>Decline</button
        >{/if}
    </div>
  {/each}
  <button
    class="primary wide"
    disabled={!preview.action || $session.pending}
    title={preview.error || 'Confirm selection'}
    onclick={() => preview.action && controller.submit(preview.action)}
  >
    {selected.length === 0 && !decision.fields.length && decision.min === 0 ? 'Skip' : 'Confirm'}<Check
      size={16}
    />
  </button>
  {#if preview.error && (selected.length > 0 || decision.fields.length)}<p class="inline-error">
      {preview.error}
    </p>{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
