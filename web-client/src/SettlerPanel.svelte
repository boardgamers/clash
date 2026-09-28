<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Footprints, Landmark, MapPin, X, ArrowRight, Check } from 'lucide-svelte';
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
</script>

<section class="action-panel floating-panel settler-panel" aria-label="Settler movement">
  <button
    class="icon-button close-action"
    aria-label="Close settler controls"
    onclick={() => controller.patch({ mode: 'overview', error: '' })}><X size={18} /></button
  >
  <h2><Footprints size={22} /> Settlers</h2>
  <p>
    {$session.view?.stopMovement
      ? 'Movement in progress. Move another eligible settler, or finish moving.'
      : 'Move settlers, then spend another action to found a city.'}
  </p>
  <div class="settler-picker">
    {#each $session.view?.settlers ?? [] as u}<button
        class:selected={u.id === unit?.id}
        onmouseenter={() => onHighlight(u.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(u.position)}
        onblur={() => onHighlight(null)}
        onclick={() => controller.patch({ selectedSettler: u.id, destination: null, error: '' })}
        ><Footprints size={15} />#{u.id + 1} · {u.position}</button
      >{/each}
  </div>
  {#if unit}<div class="settler-destinations">
      <strong><MapPin size={14} /> From {unit.position}</strong>
      <p>Choose a highlighted tile or a destination below.</p>
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
        >{:else}<p>No available destinations for this settler.</p>{/each}
    </div>
    {#if destination}<div class="settler-confirm">
        {#if Object.values(destination.payment).some((amount) => amount)}<ResourceAmount
            pile={destination.payment}
          />{/if}<button
          class="primary wide"
          disabled={$session.pending}
          onclick={() => controller.submit(destination.action)}
          >{destination.terrain === 'Unexplored' ? 'Explore' : 'Move to'}
          {destination.position}<ArrowRight size={16} /></button
        ><small
          >{$session.view?.stopMovement
            ? 'Part of the current movement action'
            : 'Costs 1 action · Up to 3 group moves'}</small
        >
        {#if destination.terrain === 'Unexplored'}<small
            >Reveals four tiles. You may need to choose their orientation.</small
          >{/if}
      </div>{/if}
    {#if !$session.view?.stopMovement}<div class="settler-found">
        <button
          class="secondary wide"
          disabled={!unit.foundAction || $session.pending}
          onclick={() => unit.foundAction && controller.submit(unit.foundAction)}
          ><Landmark size={17} /> Found city at {unit.position}</button
        ><small>{unit.foundReason ?? 'Costs 1 action · Replaces this settler with a city'}</small>
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
