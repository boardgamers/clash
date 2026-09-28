<script lang="ts">
  import { Footprints, Landmark, MapPin, X, ArrowRight, Check } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceAmount from './ResourceAmount.svelte';
  let { controller }: { controller: Controller } = $props();
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
        onclick={() => controller.patch({ selectedSettler: u.id, destination: null, error: '' })}
        ><Footprints size={15} />#{u.id + 1} · {u.position}</button
      >{/each}
  </div>
  {#if unit}<div class="settler-destinations">
      <strong><MapPin size={14} /> From {unit.position}</strong>
      <p>Choose a highlighted tile or a destination below.</p>
      {#each unit.destinations as d}<button
          class:selected={destination?.position === d.position}
          onclick={() => controller.patch({ destination: d.position, error: '' })}
          ><span>{d.position} · {typeof d.terrain === 'string' ? d.terrain : 'Exhausted'}</span
          >{#if destination?.position === d.position}<Check size={15} />{/if}</button
        >{:else}<p>No available destinations for this settler.</p>{/each}
    </div>
    {#if destination}<div class="settler-confirm">
        <ResourceAmount pile={destination.payment} /><button
          class="primary wide"
          disabled={$session.pending}
          onclick={() => controller.submit(destination.action)}
          >Move to {destination.position}<ArrowRight size={16} /></button
        ><small
          >{$session.view?.stopMovement
            ? 'Part of the current movement action'
            : 'Costs 1 action · Up to 3 group moves'}</small
        >
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
  <p class="feature-note">Settler movement currently covers revealed land without enemy units.</p>
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
