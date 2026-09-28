<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama } from 'lucide-svelte';
  import { resourceNames, type Pile, type Resource } from './types';
  let { pile }: { pile: Pile } = $props();
  const icons = {
    food: Wheat,
    wood: Trees,
    ore: Mountain,
    ideas: Lightbulb,
    gold: Coins,
    mood_tokens: Smile,
    culture_tokens: Drama,
  };
</script>

<span class="resource-amount"
  >{#each Object.entries(pile).filter(([, n]) => n) as [key, amount]}{@const Icon =
      icons[key as Resource]}<span><Icon size={14} />{amount} {resourceNames[key as Resource]}</span
    >{:else}<span>No resources</span>{/each}</span
>
