<script lang="ts">
  import type { Terrain } from './types';
  import TerrainIcon from './TerrainIcon.svelte';
  let { terrain, compact = false }: { terrain: Terrain; compact?: boolean } = $props();
</script>

<span class="exhausted-terrain"
  ><TerrainIcon {terrain} size={22} />{#if !compact}<span>Exhausted</span>{/if}</span
>
