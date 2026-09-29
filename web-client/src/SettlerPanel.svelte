<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Footprints, Landmark, X, ArrowRight, Check, Zap, Ship, Swords } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceAmount from './ResourceAmount.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import UnitIcon from './UnitIcon.svelte';
  import LeaderDetails from './LeaderDetails.svelte';
  import { terrainInfo } from './terrain';
  import { movementBonus } from './movement-bonus';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  onDestroy(() => onHighlight(null));
  const session = $derived(controller.session);
  const units = $derived($session.view?.units ?? []);
  const bonus = $derived(movementBonus($session.game));
  const movesLeft = $derived($session.view?.movementLeft ?? 0);
  const first = $derived(units.find((u) => $session.selectedUnits.includes(u.id)));
  const positions = $derived([...new Set(units.map((u) => u.position))]);
  const atPosition = $derived(units.filter((u) => u.position === first?.position));
  const destination = $derived(
    $session.moveDestination === null ? null : $session.moveDestinations[$session.moveDestination],
  );
  const settler = $derived(
    $session.selectedUnits.length === 1 ? $session.view?.settlers.find((u) => u.id === first?.id) : undefined,
  );
  const canMove = $derived(
    !!$session.view?.stopMovement || (!!$session.view?.canPlay && ($session.game?.actions_left ?? 0) > 0),
  );
  function select(id: number) {
    const selected = $session.selectedUnits.includes(id)
      ? $session.selectedUnits.filter((u) => u !== id)
      : [...$session.selectedUnits, id];
    controller.selectUnits(selected, $session.moveTarget);
  }
</script>

<section class="action-panel floating-panel settler-panel board-movement" aria-label="Unit movement">
  <button
    class="icon-button close-action"
    aria-label="Close movement controls"
    onclick={() =>
      controller.patch({
        mode: 'overview',
        tilePanel: false,
        moveTarget: null,
        moveDestination: null,
        error: '',
      })}><X size={18} /></button
  >
  <h2
    title={bonus
      ? `${bonus.source}: ${bonus.label.toLowerCase()}. Terrain restrictions still apply.`
      : 'Move up to 3 groups for 1 action. A new Move action lets units move again, unless terrain or combat prevents it.'}
  >
    <Footprints size={22} />Move{#if first}<span class="movement-origin">{first.position}</span>{/if}
  </h2>
  {#if bonus}<div class="movement-bonus"><strong>{bonus.source}</strong><span>{bonus.label}</span></div>{/if}
  {#if $session.view?.stopMovement}<div
      class="movement-markers"
      title={`${movesLeft} ${bonus ? 'bonus' : 'group'} moves remaining`}
      aria-label={`${movesLeft} ${bonus ? 'bonus' : 'group'} moves remaining`}
    >
      {#each Array.from({ length: bonus ? movesLeft : Math.max(3, movesLeft) }) as _, i}<span
          class:spent={i >= movesLeft}><Footprints size={14} /></span
        >{/each}
    </div>{/if}
  {#if positions.length > 1}<details class="movement-locations">
      <summary>Unit locations</summary>
      <div class="settler-picker" role="group" aria-label="Unit locations">
        {#each positions as position}<button
            class:selected={first?.position === position}
            aria-pressed={first?.position === position}
            onmouseenter={() => onHighlight(position)}
            onmouseleave={() => onHighlight(null)}
            onfocus={() => onHighlight(position)}
            onblur={() => onHighlight(null)}
            onclick={() => controller.selectUnits([units.find((u) => u.position === position)!.id])}
            >{position}<span class="unit-count">{units.filter((u) => u.position === position).length}</span
            ></button
          >{/each}
      </div>
    </details>{/if}
  <div class="unit-picker" role="group" aria-label="Units to move">
    {#each atPosition as u}<button
        class:selected={$session.selectedUnits.includes(u.id)}
        aria-pressed={$session.selectedUnits.includes(u.id)}
        title={`${typeof u.type === 'string' ? u.type : u.type.Leader} #${u.id + 1}${u.carrier !== null ? ` · Aboard ship #${u.carrier + 1}` : ''}`}
        onclick={() => select(u.id)}
        ><UnitIcon type={u.type} /><span>#{u.id + 1}</span>{#if u.carrier !== null}<Ship
            size={11}
          />{/if}</button
      >{/each}
  </div>
  {#each $session.view?.players
    .find((p) => p.index === $session.seat)
    ?.leaders?.filter((l) => $session.selectedUnits.includes(l.unit)) ?? [] as leader}
    <LeaderDetails
      {leader}
      compact
      actions={$session.view?.specialActions}
      pending={$session.pending}
      onUse={(action) => controller.submit(action)}
    />
  {/each}
  {#if $session.moveDestinations.length}
    <p class="movement-hint">Choose a highlighted tile.</p>
    <details class="movement-destination-list" open={$session.moveTarget !== null && !destination}>
      <summary
        >{$session.moveTarget && !destination
          ? `Choose how to enter ${$session.moveTarget}`
          : 'Destinations'}</summary
      >
      <div class="settler-destinations" role="group" aria-label={`Destinations from ${first?.position}`}>
        {#each $session.moveDestinations as d, i}{#if !$session.moveTarget || !!destination || d.position === $session.moveTarget}<button
              class:selected={$session.moveDestination === i}
              aria-pressed={$session.moveDestination === i}
              title={`${d.position} · ${terrainInfo(d.terrain).label}${d.carrier !== null ? ` · Board ship #${d.carrier + 1}` : ''}${d.attack ? ' · Attack' : ''}`}
              onmouseenter={() => onHighlight(d.position)}
              onmouseleave={() => onHighlight(null)}
              onfocus={() => onHighlight(d.position)}
              onblur={() => onHighlight(null)}
              disabled={$session.pending}
              onclick={() => controller.patch({ moveDestination: i, moveTarget: d.position })}
              ><span><TerrainIcon terrain={d.terrain} />{d.position}</span>
              {#if d.attack}<Swords size={15} />{:else if d.carrier !== null}<span
                  ><Ship size={15} /><small>#{d.carrier + 1}</small></span
                >{:else if $session.moveDestination === i}<Check size={15} />{/if}
              {#if Object.values(d.payment).some(Boolean)}<ResourceAmount pile={d.payment} />{/if}
            </button>{/if}{/each}
      </div>
    </details>
  {:else if canMove && first}<p class="settler-empty">No legal destinations for this group.</p>{/if}
  {#if destination}<div class="settler-confirm">
      {#if Object.values(destination.payment).some(Boolean)}<ResourceAmount pile={destination.payment} />{/if}
      <button
        class="primary wide"
        disabled={$session.pending}
        onclick={() => controller.submit(destination.action)}
      >
        {destination.attack
          ? 'Attack'
          : destination.carrier !== null
            ? 'Board at'
            : destination.terrain === 'Unexplored'
              ? 'Explore'
              : 'Move to'}
        {destination.position}
        {#if !$session.view?.stopMovement}<span class="settler-action-cost" aria-label="Costs 1 action"
            ><Zap size={13} />1</span
          >{/if}<ArrowRight size={16} />
      </button>
    </div>{/if}
  {#if settler?.foundAction && !$session.view?.stopMovement}<div
      class="settler-found"
      title={settler.foundReason ?? 'Costs 1 action · Replaces this settler with a city'}
    >
      <button
        class="secondary wide"
        disabled={!settler.foundAction || $session.pending}
        onclick={() => settler?.foundAction && controller.submit(settler.foundAction)}
        ><Landmark size={17} />Found city at {settler.position}<span class="settler-action-cost"
          ><Zap size={13} />1</span
        ></button
      >
    </div>{/if}
  {#if !units.length}<p>Recruit units in a city to explore.</p>{/if}
  {#if $session.view?.stopMovement}<button
      class="secondary wide finish-moving"
      disabled={$session.pending}
      onclick={() => $session.view?.stopMovement && controller.submit($session.view.stopMovement)}
      >Finish moving<Check size={16} /></button
    >{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
