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
    TriangleAlert,
  } from 'lucide-svelte';
  import type { Controller } from './controller';
  import ResourceAmount from './ResourceAmount.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import TerrainRules from './TerrainRules.svelte';
  import UnitPicker from './UnitPicker.svelte';
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
  const origin = $derived(
    $session.movingCity ?? $session.unitPosition ?? first?.position ?? units[0]?.position ?? null,
  );
  const selectedUnits = $derived(units.filter((u) => $session.selectedUnits.includes(u.id)));
  const disembarking = $derived(selectedUnits.length > 0 && selectedUnits.every((u) => u.carrier !== null));
  const movementNotes = $derived([...new Set(selectedUnits.flatMap((u) => u.movementNotes ?? []))]);
  const unitName = (u: (typeof units)[number]) =>
    typeof u.type === 'string'
      ? u.type
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

<section
  class="action-panel floating-panel settler-panel board-movement selection-tray"
  aria-label="Unit movement"
>
  <header class="movement-heading">
    <h2><Footprints size={18} />{disembarking ? 'Disembark' : 'Move'}</h2>
    {#if $session.view?.stopMovement}<span class="movement-remaining" title={bonus?.label}
        >{movesLeft} {bonus ? 'bonus' : 'group'} {movesLeft === 1 ? 'move' : 'moves'} left</span
      >{/if}
    {#if $session.view?.stopMovement}<button
        class="movement-done"
        disabled={$session.pending}
        aria-label="Finish moving"
        onclick={() => $session.view?.stopMovement && controller.submit($session.view.stopMovement)}
        >Done<Check size={14} /></button
      >{/if}
    <button
      class="icon-button close-movement"
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
  </header>
  {#if bonus}<div class="movement-bonus"><strong>{bonus.source}</strong><span>{bonus.label}</span></div>{/if}
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
          onclick={() => controller.openNomadCity(position)}><Landmark size={15} />Move city</button
        >{/each}
    </div>{/if}
  <UnitPicker
    choices={units.map((u) => ({
      id: u.id,
      type: u.type,
      player: $session.seat!,
      position: u.position,
      name: unitName(u),
      detail: u.carrier !== null ? 'Aboard ship · Disembark' : u.movementNotes?.join(' · '),
      pirate: u.pirate,
      civilization: $session.game?.players.find((p) => p.id === $session.seat)?.civilization,
    }))}
    selected={$session.selectedUnits}
    position={origin}
    pending={$session.pending}
    colorBlind={$session.colorBlind}
    playerColors={$session.playerColors}
    label="Units to move"
    onPosition={(position) => controller.focusUnitPosition(position)}
    onSelect={select}
    {onHighlight}
  />
  {#if movementNotes.length}<div class="movement-notes" role="note">
      {#each movementNotes as note}<p><TriangleAlert size={14} />{note}</p>{/each}
    </div>{/if}
  {#if selectedUnits.some((u) => typeof u.type !== 'string')}
    <details class="movement-leader-info">
      <summary>Leader abilities</summary>
      {#each $session.view?.players
        .find((p) => p.index === $session.seat)
        ?.leaders?.filter((l) => $session.selectedUnits.includes(l.unit)) ?? [] as leader}
        <LeaderDetails
          {leader}
          compact
          actions={$session.view?.specialActions}
          pending={$session.pending}
          onUse={(action, payment) => controller.submit(action, payment)}
        />
      {/each}
    </details>
  {/if}
  {#if $session.moveTarget && !destination && $session.moveDestinations.length}
    <div class="settler-destinations" role="group" aria-label="Choose how to move">
      {#each $session.moveDestinations as d, i}{#if d.position === $session.moveTarget}
          <button
            disabled={$session.pending}
            onmouseenter={() => onHighlight(d.position)}
            onmouseleave={() => onHighlight(null)}
            onclick={() => controller.patch({ moveDestination: i })}
          >
            {#if d.pirateCarrier != null}<Ship size={16} />Allied pirate
            {:else if d.carrier != null}<Ship size={16} />Board ship
            {:else if d.attack}<Swords size={16} />Attack
            {:else}<TerrainIcon terrain={d.terrain} />{terrainInfo(d.terrain).label}{/if}
            {#if Object.values(d.payment).some(Boolean)}<ResourceAmount pile={d.payment} />{/if}
          </button>
        {/if}{/each}
    </div>
  {:else if !destination}
    <p class="movement-hint">
      {!selectedUnits.length && !$session.movingCity
        ? 'Select units, then a highlighted hex.'
        : $session.moveDestinations.length
          ? disembarking
            ? 'Choose a highlighted shore.'
            : 'Choose a highlighted destination.'
          : canMove
            ? 'No legal destinations for this group.'
            : 'No moves available.'}
    </p>
  {/if}
  {#if destination && !$session.movingCity}<TerrainRules
      terrain={destination.terrain}
      notes={destination.terrainNotes}
    />{/if}
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
            ? 'Board ship'
            : destination.terrain === 'Unexplored'
              ? 'Explore'
              : disembarking
                ? 'Disembark'
                : 'Move here'}
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
        ><Landmark size={17} />Found city here<span class="settler-action-cost"
          >{#if settler.foundFree}Free{:else}<Zap size={13} />1{/if}</span
        ></button
      >
    </div>{/if}
  {#if !units.length && !$session.view?.nomadCities?.length}<p>Recruit units in a city to explore.</p>{/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>
