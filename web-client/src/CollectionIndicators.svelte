<script lang="ts">
  import { TriangleAlert } from 'lucide-svelte';
  import type { CollectionBonus, Pile } from './types';
  import { pileText } from './model';
  import { collectionTriggerLabel } from './collection-yield';
  import ResourceAmount from './ResourceAmount.svelte';
  let { bonuses, waste = {} }: { bonuses: CollectionBonus[]; waste?: Pile } = $props();
</script>

{#each bonuses as bonus}
  <span
    class="collection-trigger"
    title={collectionTriggerLabel(bonus)}
    aria-label={collectionTriggerLabel(bonus)}
  >
    <span aria-hidden="true">+</span><ResourceAmount pile={bonus.pile} />
  </span>
{/each}
{#if Object.values(waste).some(Boolean)}
  <span
    class="collection-waste"
    title={`${pileText(waste)} will be lost to the storage limit`}
    aria-label={`${pileText(waste)} will be lost to the storage limit`}
  >
    <TriangleAlert size={14} aria-hidden="true" />
  </span>
{/if}
