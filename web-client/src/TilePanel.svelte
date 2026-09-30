<script lang="ts">
  import { X, Wheat, Hammer, Users, Smile, Footprints, ArrowRight, Shield, LogOut } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { canMoveOnMap } from './map-actions';
  import { terrainInfo } from './terrain';
  import TerrainIcon from './TerrainIcon.svelte';
  import UnitIcon from './UnitIcon.svelte';
  import TileUnits from './TileUnits.svelte';
  import PirateDetails from './PirateDetails.svelte';
  import BarbarianDetails from './BarbarianDetails.svelte';
  import CityFacts from './CityFacts.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  import CityBuildings from './CityBuildings.svelte';
  import LeaderDetails from './LeaderDetails.svelte';
  let {
    controller,
    onHighlight,
  }: { controller: Controller; onHighlight: (position: string | null) => void } = $props();
  const session = $derived(controller.session);
  const position = $derived($session.focus!);
  const terrain = $derived($session.game?.map.tiles.find(([p]) => p === position)?.[1]);
  const owner = $derived($session.view?.players.find((p) => p.cities.some((c) => c.position === position)));
  const city = $derived(owner?.cities.find((c) => c.position === position));
  const publicOwner = $derived(
    $session.game?.players.find((p) => p.cities?.some((c) => c.position === position)),
  );
  const publicCity = $derived(publicOwner?.cities?.find((c) => c.position === position));
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
      <strong
        >{owner || publicOwner ? `${(owner ?? publicOwner)!.civilization} · ${position}` : position}</strong
      >
      {#if city}<CityFacts size={city.size} mood={city.mood} />{:else}<span>{terrainInfo(terrain).label}</span
        >{/if}
      <button class="icon-button" aria-label="Close tile actions" onclick={close}><X size={18} /></button>
    </header>
    {#if publicCity && publicOwner}<CityBuildings
        city={publicCity}
        owner={publicOwner.id}
        players={$session.game?.players ?? []}
      />{/if}
    {#if city?.protection}<p
        class="city-rule"
        title="Beloved: an attacking player pays these culture tokens before entering. The protection is then removed."
      >
        <Shield size={16} /> Beloved · {city.protection} culture to attack
      </p>{/if}
    {#if city?.independentPort}<p class="city-rule">
        City Independence · Port 1½ VP · Protected from influence
      </p>{/if}
    {#if city?.influenceMarker != null}<p class="city-rule">
        <Shield size={16} />
        {$session.view?.players.find((p) => p.index === city?.influenceMarker)?.civilization} · Druidic Influence
        · 1 VP
      </p>{/if}
    {#if ownCity}
      <div class="tile-city-actions" aria-label={`City ${position} actions`}>
        <button
          disabled={!!ownCity.reason || !$session.view?.canPlay || $session.pending}
          title={ownCity.reason ?? 'Collect resources · 1 action'}
          onclick={() => controller.beginCollect(position)}><Wheat size={19} />Collect</button
        >
        {#if $session.view?.nomadCities?.includes(position)}<button
            onclick={() => controller.openNomadCity(position)}><Footprints size={19} />Move city</button
          >{/if}
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
            aria-label={`${unit.carrier !== null ? 'Disembark' : 'Move'} ${typeof unit.type === 'string' ? unit.type : unit.type.Leader} #${unit.id + 1} from ${position}`}
            onclick={() => controller.openUnits([unit.id])}
          >
            <UnitIcon type={unit.type} /><span
              >{unit.carrier !== null ? 'Disembark ' : ''}{typeof unit.type === 'string'
                ? unit.type
                : ($session.view?.players
                    .find((p) => p.index === $session.seat)
                    ?.leaders?.find((l) => l.unit === unit.id)?.name ?? unit.type.Leader)} #{unit.id +
                1}</span
            >{#if unit.carrier !== null}<LogOut size={16} />{:else}<Footprints size={16} />{/if}
          </button>{/each}
      </div>
    {:else}<TileUnits players={$session.game?.players ?? []} {position} />{/if}
    {#each $session.view?.players ?? [] as player}
      {#each player.leaders?.filter((l) => l.position === position) ?? [] as leader}
        <LeaderDetails
          {leader}
          civilization={player.civilization}
          actions={player.index === $session.seat ? $session.view?.specialActions : []}
          pending={$session.pending}
          onUse={(action, payment) => controller.submit(action, payment)}
        />
      {/each}
    {/each}
    {#if $session.game}<PirateDetails game={$session.game} {position} {onHighlight} /><BarbarianDetails
        game={$session.game}
        {position}
      />{/if}
    {#if origins.length}
      <div class="tile-arrivals" role="group" aria-label={`Move to ${position}`}>
        <h3><Footprints size={15} />{terrain === 'Unexplored' ? 'Explore here' : 'Move here'}</h3>
        {#each origins as unit}<button
            aria-label={`${unit.carrier !== null ? 'Disembark' : 'Move'} ${typeof unit.type === 'string' ? unit.type : unit.type.Leader} #${unit.id + 1} from ${unit.position} to ${position}`}
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
