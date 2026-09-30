<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    Footprints,
    Landmark,
    X,
    ArrowRight,
    Check,
    Zap,
    Ship,
    Swords,
    LogOut,
    TriangleAlert,
  } from 'lucide-svelte';
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
  const atPosition = $derived(units.filter((u) => u.position === ($session.movingCity ?? first?.position)));
  const passengers = $derived(atPosition.filter((u) => u.carrier !== null));
  const selectedUnits = $derived(units.filter((u) => $session.selectedUnits.includes(u.id)));
  const disembarking = $derived(selectedUnits.length > 0 && selectedUnits.every((u) => u.carrier !== null));
  const movementNotes = $derived([...new Set(selectedUnits.flatMap((u) => u.movementNotes ?? []))]);
  const unitName = (u: (typeof units)[number]) =>
    typeof u.type === 'string'
      ? `${u.type} #${u.id + 1}`
      : ($session.view?.players.find((p) => p.index === $session.seat)?.leaders?.find((l) => l.unit === u.id)
          ?.name ?? u.type.Leader);
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
      : [
          ...$session.selectedUnits.filter(
            (selected) =>
              (units.find((u) => u.id === selected)?.type === 'Ship') ===
              (units.find((u) => u.id === id)?.type === 'Ship'),
          ),
          id,
        ];
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
    <Footprints size={22} />{disembarking ? 'Disembark' : 'Move'}{#if $session.movingCity || first}<span
        class="movement-origin">{$session.movingCity ?? first?.position}</span
      >{/if}
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
  {#if positions.length > 1}<div class="settler-picker" role="group" aria-label="Unit locations">
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
    </div>{/if}
  {#if $session.view?.nomadCities?.length}<div
      class="settler-picker"
      role="group"
      aria-label="Nomadic cities"
    >
      {#each $session.view.nomadCities as position}<button
          class:selected={$session.movingCity === position}
          aria-pressed={$session.movingCity === position}
          onmouseenter={() => onHighlight(position)}
          onmouseleave={() => onHighlight(null)}
          onclick={() => controller.openNomadCity(position)}><Landmark size={15} />{position}</button
        >{/each}
    </div>{/if}
  <div class="unit-picker" role="group" aria-label="Units to move">
    {#each atPosition.filter((u) => u.carrier === null) as u}<button
        class:selected={$session.selectedUnits.includes(u.id)}
        aria-pressed={$session.selectedUnits.includes(u.id)}
        title={`${typeof u.type === 'string' ? u.type : u.type.Leader} #${u.id + 1}${u.carrier !== null ? ` · Aboard ship #${u.carrier + 1}` : ''}`}
        onclick={() => select(u.id)}
        ><UnitIcon type={u.type} /><span>#{u.id + 1}</span>{#if u.carrier !== null}<Ship
            size={11}
          />{/if}</button
      >{/each}
  </div>
  {#if passengers.length}<div class="movement-passengers" role="group" aria-label="Passengers">
      {#each passengers as u}<button
          class="secondary"
          class:selected={$session.selectedUnits.includes(u.id)}
          aria-pressed={$session.selectedUnits.includes(u.id)}
          title={`Aboard ship #${u.carrier! + 1}`}
          onclick={() => select(u.id)}
        >
          <LogOut size={16} /><UnitIcon type={u.type} />Disembark {unitName(u)}
        </button>{/each}
    </div>{/if}
  {#if movementNotes.length}<div class="movement-notes" role="note">
      {#each movementNotes as note}<p><TriangleAlert size={14} />{note}</p>{/each}
    </div>{/if}
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
    <p class="movement-hint">
      {disembarking ? 'Choose a highlighted shore tile.' : 'Choose a highlighted tile.'}
    </p>
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
              title={`${d.position} · ${terrainInfo(d.terrain).label}${d.carrier != null ? ` · Board ship #${d.carrier + 1}` : ''}${d.attack ? ' · Attack' : ''}`}
              onmouseenter={() => onHighlight(d.position)}
              onmouseleave={() => onHighlight(null)}
              onfocus={() => onHighlight(d.position)}
              onblur={() => onHighlight(null)}
              disabled={$session.pending}
              onclick={() => controller.patch({ moveDestination: i, moveTarget: d.position })}
              ><span><TerrainIcon terrain={d.terrain} />{d.position}</span>
              {#if d.pirateCarrier != null}<span><Ship size={15} /><small>Allied pirate</small></span
                >{:else if d.attack}<Swords size={15} />{:else if d.carrier != null}<span
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
          : destination.carrier != null || destination.pirateCarrier != null
            ? 'Board at'
            : destination.terrain === 'Unexplored'
              ? 'Explore'
              : disembarking
                ? 'Disembark at'
                : 'Move to'}
        {destination.position}
        {#if !$session.view?.stopMovement}<span class="settler-action-cost" aria-label="Costs 1 action"
            ><Zap size={13} />1</span
          >{/if}<ArrowRight size={16} />
      </button>
    </div>{/if}
  {#if settler?.foundAction && !$session.view?.stopMovement}<div
      class="settler-found"
      title={settler.foundReason ??
        (settler.foundFree ? 'Founder · Free action' : 'Costs 1 action · Replaces this settler with a city')}
    >
      <button
        class="secondary wide"
        disabled={!settler.foundAction || $session.pending}
        onclick={() => settler?.foundAction && controller.submit(settler.foundAction)}
        ><Landmark size={17} />Found city at {settler.position}<span class="settler-action-cost"
          >{#if settler.foundFree}Free{:else}<Zap size={13} />1{/if}</span
        ></button
      >
    </div>{/if}
  {#if !units.length && !$session.view?.nomadCities?.length}<p>Recruit units in a city to explore.</p>{/if}
  {#if $session.view?.stopMovement}<button
      class="secondary wide finish-moving"
      disabled={$session.pending}
      onclick={() => $session.view?.stopMovement && controller.submit($session.view.stopMovement)}
      >Finish moving<Check size={16} /></button
    >{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
