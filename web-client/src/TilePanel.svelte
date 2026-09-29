<script lang="ts">
  import { X, Wheat, Hammer, Users, Smile, Footprints, ArrowRight } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { canMoveOnMap } from './map-actions';
  import { terrainInfo } from './terrain';
  import TerrainIcon from './TerrainIcon.svelte';
  import UnitIcon from './UnitIcon.svelte';
  import TileUnits from './TileUnits.svelte';
  import CityFacts from './CityFacts.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  const position = $derived($session.focus!);
  const terrain = $derived($session.game?.map.tiles.find(([p]) => p === position)?.[1]);
  const owner = $derived($session.view?.players.find((p) => p.cities.some((c) => c.position === position)));
  const city = $derived(owner?.cities.find((c) => c.position === position));
  const ownCity = $derived($session.view?.cities.find((c) => c.position === position));
  const ownUnits = $derived($session.view?.units?.filter((u) => u.position === position) ?? []);
  const canMove = $derived(canMoveOnMap($session.view, $session.game));
  const origins = $derived.by(() => {
    // Recompute when the filtered game state changes, never from hidden state.
    $session.view;
    return controller.moveOrigins(position);
  });
  const collectors = $derived(
    $session.view?.canPlay
      ? $session.view.cities.filter(
          (c) => !c.reason && c.choices.some((choice) => choice.position === position),
        )
      : [],
  );
  function close() {
    onHighlight(null);
    controller.patch({ tilePanel: false });
  }
</script>

{#if terrain}
  <section class="board-context" aria-label={`Tile ${position} actions`}>
    <header>
      <TerrainIcon {terrain} size={21} />
      <strong>{owner ? `${owner.civilization} · ${position}` : position}</strong>
      {#if city}<CityFacts size={city.size} mood={city.mood} />{:else}<span>{terrainInfo(terrain).label}</span
        >{/if}
      <button class="icon-button" aria-label="Close tile actions" onclick={close}><X size={18} /></button>
    </header>
    {#if ownCity}
      <div class="tile-city-actions" aria-label={`City ${position} actions`}>
        <button
          disabled={!!ownCity.reason || !$session.view?.canPlay || $session.pending}
          title={ownCity.reason ?? 'Collect resources · 1 action'}
          onclick={() => controller.beginCollect(position)}><Wheat size={19} />Collect</button
        >
        <button onclick={() => controller.openCities(position, 'build')}><Hammer size={19} />Build</button>
        <button onclick={() => controller.openCities(position, 'recruit')}><Users size={19} />Recruit</button>
        <button onclick={() => controller.openCities(position, 'happiness')}
          ><Smile size={19} />Happiness</button
        >
      </div>
      {#if ownCity.activations > 0}<ActivationStatus city={ownCity} warning />{/if}
    {/if}
    {#if ownUnits.length && canMove}
      <div class="tile-unit-actions" role="group" aria-label={`Move units at ${position}`}>
        {#each ownUnits as unit}<button
            aria-label={`Move ${typeof unit.type === 'string' ? unit.type : unit.type.Leader} #${unit.id + 1} from ${position}`}
            onclick={() => controller.openUnits([unit.id])}
          >
            <UnitIcon type={unit.type} /><span
              >{typeof unit.type === 'string' ? unit.type : unit.type.Leader} #{unit.id + 1}</span
            ><Footprints size={16} />
          </button>{/each}
      </div>
    {:else}<TileUnits players={$session.game?.players ?? []} {position} />{/if}
    {#if origins.length}
      <div class="tile-arrivals" role="group" aria-label={`Move to ${position}`}>
        <h3><Footprints size={15} />{terrain === 'Unexplored' ? 'Explore here' : 'Move here'}</h3>
        {#each origins as unit}<button
            aria-label={`Move ${typeof unit.type === 'string' ? unit.type : unit.type.Leader} #${unit.id + 1} from ${unit.position} to ${position}`}
            onmouseenter={() => onHighlight(unit.position)}
            onmouseleave={() => onHighlight(null)}
            onfocus={() => onHighlight(unit.position)}
            onblur={() => onHighlight(null)}
            onclick={() => {
              onHighlight(null);
              controller.openUnits([unit.id], position);
            }}
          >
            <UnitIcon type={unit.type} /><span>#{unit.id + 1} · {unit.position}</span><ArrowRight
              size={14}
            />{position}
          </button>{/each}
      </div>
    {/if}
    {#if !ownCity && collectors.length}<div
        class="tile-harvest"
        role="group"
        aria-label={`Collect at ${position}`}
      >
        {#each collectors as c}<button
            onclick={() => controller.collectFromTile(c.position, position)}
            onmouseenter={() => onHighlight(c.position)}
            onmouseleave={() => onHighlight(null)}
            onfocus={() => onHighlight(c.position)}
            onblur={() => onHighlight(null)}
          >
            <Wheat size={16} />Collect with {c.position}<ArrowRight size={14} />
          </button>{/each}
      </div>{/if}
  </section>
{/if}
