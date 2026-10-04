<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link, Check } from 'lucide-svelte';
  import type { CollectionBonus as Bonus, Pile, Resource } from './types';
  import CollectionIndicators from './CollectionIndicators.svelte';
  let {
    piles,
    selected,
    bonuses = [],
    waste = [],
  }: { piles: Pile[]; selected: boolean; bonuses?: Bonus[][]; waste?: Pile[] } = $props();
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

{#each piles as pile, index}
  {#if index > 0}<span class="collection-or">/</span>{/if}
  {#each Object.entries(pile).filter(([, n]) => n) as [resource, amount]}{@const Icon =
      icons[resource as Resource]}
    {#each Array.from({ length: Math.min(amount!, 3) }) as _}<Icon size={15} aria-hidden="true" />{/each}
    {#if amount! > 3}<b>{amount}</b>{/if}
  {/each}
  <CollectionIndicators bonuses={bonuses[index] ?? []} waste={waste[index]} />
{/each}
{#if selected}<Check size={14} class="collection-check" aria-hidden="true" />{/if}
