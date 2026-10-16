<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ChevronDown } from 'lucide-svelte';
  import { positionMapGuide } from './strategy-key';
  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<details open class="map-tooltip pirate-spawn-key collapsible-map-guide" use:positionMapGuide>
  <summary><strong>{title}</strong><ChevronDown size={18} /></summary>
  {@render children()}
</details>
