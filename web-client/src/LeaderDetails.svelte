<script lang="ts">
  import { Crown, Sparkles, ChevronRight } from 'lucide-svelte';
  import type { PlayerView } from './types';
  import ResourceText from './ResourceText.svelte';
  let {
    leader,
    civilization,
    compact = false,
  }: {
    leader: NonNullable<PlayerView['leaders']>[number];
    civilization?: string;
    compact?: boolean;
  } = $props();
</script>

<details class="tile-leader" open={!compact}>
  <summary
    ><Crown size={17} /><strong>{leader.name}</strong>{#if civilization}<small>{civilization}</small
      >{/if}<ChevronRight size={14} /></summary
  >
  {#each leader.abilities as ability}
    <div class="tile-leader-ability">
      <strong><Sparkles size={13} />{ability.name}</strong>
      <p><ResourceText text={ability.description} /></p>
    </div>
  {/each}
</details>
