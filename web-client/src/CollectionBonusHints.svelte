<script lang="ts">
  import ResourceAmount from './ResourceAmount.svelte';
  import type { CityView, Selection, Pile, Resource } from './types';
  import { collectionYield } from './collection-yield';
  let { city, selection }: { city: CityView; selection: Selection[] } = $props();
  const total = $derived.by(() => {
    const result: Pile = {};
    for (const choice of selection)
      for (const [resource, amount] of Object.entries(collectionYield(choice, selection)))
        result[resource as Resource] = (result[resource as Resource] ?? 0) + amount!;
    return result;
  });
</script>

{#each city.collectionBonuses ?? [] as bonus}
  {@const ready = Object.entries(bonus.minimum).every(([r, n]) => (total[r as Resource] ?? 0) >= n!)}
  <div class="collection-bonus-hint" class:ready role="status">
    <strong>{bonus.source}</strong><span
      >Collect <ResourceAmount pile={bonus.minimum} compact /> → <ResourceAmount
        pile={bonus.pile}
        compact
      /></span
    >
    <small
      >{ready
        ? 'Bonus included'
        : Object.entries(bonus.minimum)
            .map(([r, n]) => `${Math.min(total[r as Resource] ?? 0, n!)}/${n}`)
            .join(' · ') + ' selected · once per turn'}</small
    >
  </div>
{/each}
