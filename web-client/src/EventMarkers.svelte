<script lang="ts">
  import { ScrollText } from 'lucide-svelte';
  let { remaining }: { remaining: number } = $props();
  let label = $derived(`${remaining} event marker${remaining === 1 ? '' : 's'} left`);
</script>

<span
  class="event-markers"
  role="img"
  aria-label={label}
  title={`${label}. Research removes a marker; removing the last triggers an event, then refills all three.`}
>
  <ScrollText size={12} />
  {#each [0, 1, 2] as index}<i class:filled={index < remaining}></i>{/each}
</span>
