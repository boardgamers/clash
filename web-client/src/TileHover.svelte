<script lang="ts">
  import type { Readable } from 'svelte/store';
  import { Skull, Landmark } from 'lucide-svelte';
  import type { TileHoverData } from './tile-hover';
  import { playerColor, playerSymbol } from './types';
  import { terrainInfo } from './terrain';
  import TerrainIcon from './TerrainIcon.svelte';
  import UnitIcon from './UnitIcon.svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  let { data }: { data: Readable<TileHoverData | null> } = $props();
</script>

{#if $data}
  {#if $data.city}
    <strong class="tile-hover-city"
      ><Landmark size={15} />{#if $data.colorBlind}{playerSymbol($data.city.owner, $data.playerSymbols)}
      {/if}{$data.city.civilization} city<small>{$data.city.mood}</small></strong
    >
  {:else}<strong class="tile-hover-terrain"
      ><TerrainIcon terrain={$data.terrain} size={15} />{terrainInfo($data.terrain).label}</strong
    >{/if}
  {#each $data.stacks as { player, groups }}
    <div
      class="tile-hover-owner"
      style={`--owner:${playerColor(player.id, $data.colorBlind, $data.playerColors)}`}
    >
      {#if player.id !== $data.city?.owner}<strong
          ><CivilizationEmblem
            civilization={player.civilization}
            size={15}
          />{#if $data.colorBlind}{playerSymbol(player.id, $data.playerSymbols)}
          {/if}{player.civilization}</strong
        >{/if}
      {#each [false, true] as aboard}
        {@const visible = groups.filter((group) => group.aboard === aboard)}
        {#if visible.length}
          <div class="tile-hover-units">
            {#if aboard}<small>Aboard</small>{/if}
            {#each visible as group}
              <span
                >{#if group.pirate}<Skull size={15} />{:else}<UnitIcon type={group.type} size={15} />{/if}<b
                  >{group.count}</b
                >
                {group.name}</span
              >
            {/each}
          </div>
        {/if}
      {/each}
    </div>
  {:else}{#if $data.terrain !== 'Unexplored'}<small class="tile-hover-empty">No units</small>{/if}{/each}
{/if}
