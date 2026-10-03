<script lang="ts">
  import type { Terrain } from './types';
  import TerrainIcon from './TerrainIcon.svelte';
  import { terrainInfo, terrainMovementRule } from './terrain';
  let {
    terrain,
    notes,
    showHeading = true,
  }: { terrain: Terrain; notes?: string[]; showHeading?: boolean } = $props();
  const rule = $derived(terrainMovementRule(terrain));
</script>

{#if rule}<aside class="terrain-rules" aria-label={`${terrainInfo(terrain).label} rules`}>
    {#if showHeading}<TerrainIcon {terrain} size={21} />{/if}
    <div>
      {#if showHeading}<strong>{terrainInfo(terrain).label}</strong>{/if}
      {#each notes?.length ? notes : [rule] as note}<p>{note}</p>{/each}
      <small>No terrain bonus to combat value.</small>
    </div>
  </aside>{/if}
