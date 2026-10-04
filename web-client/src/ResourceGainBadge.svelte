<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link } from 'lucide-svelte';
  import type { ResourceMarker } from './resource-playback';
  import { resourceNames } from './types';
  let {
    markers,
    animate,
    transient,
    civilization,
  }: { markers: ResourceMarker[]; animate: boolean; transient: boolean; civilization: string } = $props();
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
    captives: Link,
  };
</script>

<div
  class="resource-gain-badge"
  class:rise={animate}
  class:transient
  role="img"
  aria-label={`${civilization}: gained ${markers.map((m) => `${m.amount} ${resourceNames[m.resource].toLowerCase()}`).join(', ')} at ${markers[0].position}`}
>
  {#each markers as marker}
    {@const Icon = icons[marker.resource]}
    <span class="gain {marker.resource}"
      ><b>+{marker.amount}</b><Icon size={20} strokeWidth={1.8} aria-hidden="true" /></span
    >
  {/each}
</div>
