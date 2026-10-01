<script lang="ts">
  import type { Terrain } from './types';
  import TerrainIcon from './TerrainIcon.svelte';
  import { terrainInfo, terrainMovementRule } from './terrain';
  let { terrain, notes }: { terrain: Terrain; notes?: string[] } = $props();
  const rule = $derived(terrainMovementRule(terrain));
</script>

{#if rule}<aside class="terrain-rules" aria-label={`${terrainInfo(terrain).label} rules`}>
    <TerrainIcon {terrain} size={21} />
    <div>
      <strong>{terrainInfo(terrain).label}</strong>
      {#each notes?.length ? notes : [rule] as note}<p>{note}</p>{/each}
      <small>No terrain bonus to combat value.</small>
    </div>
  </aside>{/if}
