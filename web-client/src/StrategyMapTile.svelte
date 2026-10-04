<script lang="ts">
  import { Swords, Ship, Footprints, Landmark, Shield, Skull } from 'lucide-svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import ExhaustedTerrain from './ExhaustedTerrain.svelte';
  import { playerColor, playerSymbol } from './types';
  import type { StrategyTile } from './strategy';
  let {
    tile,
    colorBlind,
    playerColors,
    playerSymbols,
  }: { tile: StrategyTile; colorBlind: boolean; playerColors: string[]; playerSymbols: string[] } = $props();
</script>

{#if typeof tile.terrain !== 'string'}<ExhaustedTerrain terrain={tile.terrain} />{:else}
  <span class="strategy-terrain" class:important={tile.terrain === 'Forest' || tile.terrain === 'Mountain'}
    ><TerrainIcon
      terrain={tile.terrain}
      size={tile.terrain === 'Forest' || tile.terrain === 'Mountain' ? 30 : 18}
    /></span
  >
{/if}
{#each tile.occupants as occupant}
  <span class="strategy-owner" style={`--owner:${playerColor(occupant.player.id, colorBlind, playerColors)}`}>
    {#if colorBlind}<span class="strategy-symbol">{playerSymbol(occupant.player.id, playerSymbols)}</span
      >{/if}
    {#if occupant.city}<span class="strategy-city"
        ><Landmark size={17} />{occupant.size}{#if occupant.city.city_pieces?.fortress != null}<Shield
            size={16}
          />{/if}</span
      >{/if}
    {#if occupant.army || occupant.ships || occupant.settlers}<span class="strategy-forces">
        {#if occupant.army - occupant.aboard > 0}<span
            ><Swords size={18} /><b>{occupant.army - occupant.aboard}</b></span
          >{/if}
        {#if occupant.ships}<span
            >{#if occupant.player.civilization === 'Pirates'}<Skull size={18} />{:else}<Ship
                size={18}
              />{/if}<b>{occupant.ships}</b></span
          >{/if}
        {#if occupant.aboard}<span class="strategy-aboard"
            ><Swords size={14} />{occupant.aboard}<small>aboard</small></span
          >{/if}
        {#if occupant.settlers}<span class="strategy-settlers"
            ><Footprints size={15} />{occupant.settlers}</span
          >{/if}
      </span>{/if}
  </span>
{/each}
