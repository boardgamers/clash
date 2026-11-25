<script lang="ts">
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import OwnershipSymbol from './OwnershipSymbol.svelte';
  import { emblemInkColor, type CityInfluencer } from './building-owner';
  import { playerColor, playerSymbol } from './types';

  let {
    influencers,
    colorBlind = false,
    playerColors = [],
    playerSymbols = [],
    compact = false,
  }: {
    influencers: readonly CityInfluencer[];
    colorBlind?: boolean;
    playerColors?: readonly string[];
    playerSymbols?: readonly string[];
    compact?: boolean;
  } = $props();
</script>

<span class="influence-medallions" class:compact aria-hidden="true">
  {#each influencers as influencer (influencer.owner)}
    {@const bg = playerColor(influencer.owner, colorBlind, playerColors)}
    {@const ink = emblemInkColor(bg)}
    <span class="influence-medallion" style={`--medallion-bg:${bg};--medallion-ink:${ink}`}>
      <CivilizationEmblem civilization={influencer.civilization} size={compact ? 11 : 13} />
      {#if colorBlind && !['Barbarians', 'Pirates'].includes(influencer.civilization)}
        {@const symbol = playerSymbol(influencer.owner, playerSymbols)}
        <span class="influence-symbol" data-symbol={symbol}>
          <OwnershipSymbol {symbol} size={compact ? 7 : 8} />
        </span>
      {/if}
      {#if influencer.count > 1}
        <span class="influence-count">{influencer.count}</span>
      {/if}
    </span>
  {/each}
</span>
