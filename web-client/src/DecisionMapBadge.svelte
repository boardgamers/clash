<script lang="ts">
  import { Landmark } from 'lucide-svelte';
  import type { Decision } from './types';
  import UnitIcon from './UnitIcon.svelte';
  let { option }: { option: Decision['options'][number] } = $props();
</script>

{#if option.mapTarget?.kind === 'unit'}
  <UnitIcon type={option.mapTarget.unitType} size={19} />
  <span>#{option.mapTarget.unit + 1}</span>
{:else}
  <Landmark size={17} />
  <span>{option.name.split(' · ')[0]}</span>
{/if}
