<script lang="ts">
  import { Layers2, Skull } from 'lucide-svelte';
  import BarbarianIcon from './BarbarianIcon.svelte';
  import UnitIcon from './UnitIcon.svelte';
  let {
    groups,
    symbol,
    pirate = false,
    barbarian = false,
  }: {
    groups: { type: string | { Leader: string }; count: number }[];
    symbol: string;
    pirate?: boolean;
    barbarian?: boolean;
  } = $props();
</script>

{#if symbol}<span class="unit-owner-symbol" aria-hidden="true">{symbol}</span>{/if}
<span class="unit-map-types" aria-hidden="true">
  {#if pirate}<Skull size={14} />{:else if barbarian}<BarbarianIcon
      size={14}
    />{:else if groups.length > 1}<Layers2 size={14} />{:else}<UnitIcon
      type={groups[0].type}
      size={14}
    />{/if}
  <b>{groups.reduce((sum, group) => sum + group.count, 0)}</b>
</span>
