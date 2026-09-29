<script lang="ts">
  import { Layers2 } from 'lucide-svelte';
  import UnitIcon from './UnitIcon.svelte';
  let { groups, symbol }: { groups: { type: string | { Leader: string }; count: number }[]; symbol: string } =
    $props();
</script>

{#if symbol}<span class="unit-owner-symbol" aria-hidden="true">{symbol}</span>{/if}
<span class="unit-map-types" aria-hidden="true">
  {#if groups.length > 1}<Layers2 size={14} />{:else}<UnitIcon type={groups[0].type} size={14} />{/if}
  <b>{groups.reduce((sum, group) => sum + group.count, 0)}</b>
</span>
