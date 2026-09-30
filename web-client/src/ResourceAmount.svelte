<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link } from 'lucide-svelte';
  import { resourceNames, type Pile, type Resource } from './types';
  let {
    pile,
    compact = true,
    showZero = false,
  }: { pile: Pile; compact?: boolean; showZero?: boolean } = $props();
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

<span class="resource-amount"
  >{#each Object.entries(pile).filter(([, n]) => showZero || n) as [key, amount]}{@const Icon =
      icons[key as Resource]}<span
      title={`${amount} ${resourceNames[key as Resource]}`}
      aria-label={compact ? `${amount} ${resourceNames[key as Resource]}` : undefined}
      ><Icon size={14} aria-hidden="true" /><span>{amount}</span>{#if !compact}<span
          >{resourceNames[key as Resource]}</span
        >{/if}</span
    >{:else}<span>No resources</span>{/each}</span
>
