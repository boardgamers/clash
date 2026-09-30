<script lang="ts">
  import { Landmark, Flower2 } from 'lucide-svelte';
  import vikings from '../../client/assets/viking-ship-svgrepo-com.png';
  import rome from '../../client/assets/colosseum-rome-svgrepo-com.png';
  import greece from '../../client/assets/temple-building-with-columns-svgrepo-com.png';
  import china from '../../client/assets/great-wall-of-china-chinese-svgrepo-com.png';
  import pirates from '../../client/assets/pirate-symbol-mark-svgrepo-com.png';
  import barbarians from '../../client/assets/warrior-svgrepo-com.png';
  let { civilization, size = 22 }: { civilization: string; size?: number } = $props();
  const emblems: Record<string, string> = {
    Vikings: vikings,
    Rome: rome,
    Greece: greece,
    China: china,
    Pirates: pirates,
    Barbarians: barbarians,
  };
</script>

{#if emblems[civilization]}
  <span
    aria-hidden="true"
    class="civilization-emblem"
    style={`--emblem:url("${emblems[civilization]}");width:${size}px;height:${size}px`}
  ></span>
{:else if civilization === 'India'}<Flower2 {size} aria-hidden="true" />{:else}<Landmark
    {size}
    aria-hidden="true"
  />{/if}
