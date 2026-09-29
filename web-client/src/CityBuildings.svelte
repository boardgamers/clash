<script lang="ts">
  import { Landmark, Crown } from 'lucide-svelte';
  import { buildingInfo } from './city';
  import type { City, Player } from './types';
  let { city, owner, players }: { city: City; owner: number; players: Player[] } = $props();
  const buildings = $derived(
    Object.entries(city.city_pieces ?? {}).flatMap(([key, player]) => {
      if (typeof player !== 'number') return [];
      const name = key[0].toUpperCase() + key.slice(1);
      return [
        {
          name,
          player,
          info: buildingInfo[name],
          civilization: players.find((p) => p.id === player)?.civilization,
        },
      ];
    }),
  );
</script>

<ul class="city-buildings" aria-label={`Buildings at ${city.position}`}>
  <li title="City center"><Landmark size={16} /><span>Settlement</span></li>
  {#each buildings as building}
    {@const Icon = building.info?.icon ?? Landmark}
    <li title={building.info?.effect} class:foreign-building={building.player !== owner}>
      <Icon size={16} /><span>{building.name}</span>
      {#if building.player !== owner}<small
          >· {building.civilization ?? `Player ${building.player + 1}`}</small
        >{/if}
    </li>
  {/each}
  {#each city.city_pieces?.wonders ?? [] as wonder}
    <li class="built-wonder"><Crown size={16} /><span>{wonder.replace(/([a-z])([A-Z])/g, '$1 $2')}</span></li>
  {/each}
</ul>
