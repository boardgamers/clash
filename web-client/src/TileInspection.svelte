<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { Controller } from './controller';
  import TerrainIcon from './TerrainIcon.svelte';
  import TerrainRules from './TerrainRules.svelte';
  import CityBuildings from './CityBuildings.svelte';
  import TileUnits from './TileUnits.svelte';
  import UnitIcon from './UnitIcon.svelte';
  import { terrainInfo } from './terrain';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const position = $derived($session.focus!);
  const raided = $derived(!$session.playback && $session.tradeRoutes && $session.mode === 'overview' &&
    $session.view?.tradeRoutes?.find(p => p.player === ($session.tradeRoutePlayer ?? $session.seat ?? $session.view?.activePlayer))
      ?.raidedSettlers?.some(source => source.position === position));
  const frame = $derived($session.playback?.frame);
  const players = $derived(frame?.players ?? $session.game?.players ?? []);
  const terrain = $derived(
    (frame?.tiles ?? $session.game?.map.tiles ?? []).find(([p]) => p === position)?.[1],
  );
  const owner = $derived(players.find((p) => p.cities?.some((c) => c.position === position)));
  const city = $derived(owner?.cities?.find((c) => c.position === position));
</script>

{#if terrain}
  <section class="board-context" aria-label={`Inspect tile ${position}`}>
    <header>
      <TerrainIcon {terrain} size={21} />
      <strong>{owner ? `${owner.civilization} city` : terrainInfo(terrain).label}</strong>
      <small class="tile-coordinate">{position}</small>
      <button
        class="icon-button"
        data-panel-close aria-label="Close tile details"
        onclick={() => controller.patch({ tilePanel: false })}><X size={18} /></button
      >
    </header>
    {#if terrain !== 'Unexplored'}
      {#if city && owner}
        <p>{city.mood_state}</p>
        <CityBuildings {city} owner={owner.id} {players} />
      {/if}
      <TileUnits {players} {position} />
      {#if raided}<p class="city-rule raided-settlers-note"><UnitIcon type="Settler" size={16} />Settlers blocked by Huns’ Raiders</p>{/if}
    {/if}
    <TerrainRules {terrain} />
  </section>
{/if}
