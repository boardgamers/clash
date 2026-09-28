<script lang="ts">
  import type { Terrain } from './types';
  import { terrainInfo } from './terrain';
  let { terrain, size = 24 }: { terrain: Terrain; size?: number } = $props();
  const info = $derived(terrainInfo(terrain));
</script>

<svg class="terrain-icon" width={size} height={size} viewBox="0 0 28 28" role="img" aria-label={info.label}>
  <title>{info.label}</title>
  <polygon points="27,14 20.5,25.3 7.5,25.3 1,14 7.5,2.7 20.5,2.7" fill={info.color} />
  <g transform="translate(6 6)"><info.Icon size={16} strokeWidth={2} color="#263f33" /></g>
  {#if info.exhausted}<path d="M5 23 23 5" stroke="#263f33" stroke-width="2" />{/if}
</svg>
