<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Footprints, Landmark, X, ArrowRight, Check, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceAmount from './ResourceAmount.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import { terrainInfo } from './terrain';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  onDestroy(() => onHighlight(null));
  const session = $derived(controller.session);
  let unit = $derived($session.view?.settlers.find((u) => u.id === $session.selectedSettler));
  let destination = $derived(unit?.destinations.find((d) => d.position === $session.destination));
  let canMove = $derived(
    !!$session.view?.stopMovement || (!!$session.view?.canPlay && ($session.game?.actions_left ?? 0) > 0),
  );
</script>

<section class="action-panel floating-panel settler-panel" aria-label="Settler movement">
  <button
    class="icon-button close-action"
    aria-label="Close settler controls"
    onclick={() => controller.patch({ mode: 'overview', error: '' })}><X size={18} /></button
  >
  <h2 title="Move settlers, then spend another action to found a city."><Footprints size={22} /> Settlers</h2>
  <div class="settler-picker">
    {#each $session.view?.settlers ?? [] as u}<button
        class:selected={u.id === unit?.id}
        aria-pressed={u.id === unit?.id}
        title={`Locate settler #${u.id + 1} at ${u.position}`}
        onmouseenter={() => onHighlight(u.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(u.position)}
        onblur={() => onHighlight(null)}
        onclick={() => controller.patch({ selectedSettler: u.id, destination: null, error: '' })}
        ><Footprints size={15} />#{u.id + 1} · {u.position}</button
      >{/each}
  </div>
  {#if unit}
    {#if unit.destinations.length}<div
        class="settler-destinations"
        role="group"
        aria-label={`Destinations from ${unit.position}`}
      >
        {#each unit.destinations as d}<button
            class:selected={destination?.position === d.position}
            title={`${d.position} · ${terrainInfo(d.terrain).label}`}
            aria-label={`${d.position} · ${terrainInfo(d.terrain).label}`}
            aria-pressed={destination?.position === d.position}
            onmouseenter={() => onHighlight(d.position)}
            onmouseleave={() => onHighlight(null)}
            onfocus={() => onHighlight(d.position)}
            onblur={() => onHighlight(null)}
            disabled={$session.pending}
            onclick={() => controller.patch({ destination: d.position, error: '' })}
            ><span><TerrainIcon terrain={d.terrain} />{d.position}</span
            >{#if destination?.position === d.position}<Check size={15} />{/if}</button
          >{/each}
      </div>{:else if canMove}<p class="settler-empty">No legal moves from {unit.position}.</p>{/if}
    {#if destination}<div class="settler-confirm">
        {#if Object.values(destination.payment).some((amount) => amount)}<ResourceAmount
            pile={destination.payment}
          />{/if}<button
          class="primary wide"
          title={destination.terrain === 'Unexplored'
            ? 'Reveal four tiles and choose their orientation if needed.'
            : $session.view?.stopMovement
              ? 'Part of the current movement action.'
              : 'Costs 1 action · Up to 3 group moves.'}
          disabled={$session.pending}
          onclick={() => controller.submit(destination.action)}
          >{destination.terrain === 'Unexplored' ? 'Explore' : 'Move to'}
          {destination.position}{#if !$session.view?.stopMovement}<span
              class="settler-action-cost"
              aria-label="Costs 1 action"><Zap size={13} />1</span
            >{/if}<ArrowRight size={16} /></button
        >
      </div>{/if}
    {#if !$session.view?.stopMovement}<div
        class="settler-found"
        title={unit.foundReason ?? 'Costs 1 action · Replaces this settler with a city'}
      >
        <button
          class="secondary wide"
          disabled={!unit.foundAction || $session.pending}
          onclick={() => unit.foundAction && controller.submit(unit.foundAction)}
          ><Landmark size={17} /> Found city at {unit.position}<span
            class="settler-action-cost"
            aria-label="Costs 1 action"><Zap size={13} />1</span
          ></button
        >
      </div>{/if}
  {:else}<p>Recruit a settler in one of your cities to expand.</p>{/if}
  {#if $session.view?.stopMovement}<button
      class="primary wide"
      disabled={$session.pending}
      onclick={() => $session.view?.stopMovement && controller.submit($session.view.stopMovement)}
      >Finish moving <Check size={16} /></button
    >{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
