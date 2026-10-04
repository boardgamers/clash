<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link } from 'lucide-svelte';
  import { journalParts } from './model';
  import { researchTextParts, type ResearchReference } from './research-links';
  let {
    text,
    positions,
    onCoordinate,
    onLocate,
    research = [],
    onResearch,
  }: {
    text: string;
    positions?: Set<string>;
    onCoordinate?: (position: string | null) => void;
    onLocate?: (position: string) => void;
    research?: ResearchReference[];
    onResearch?: (reference: ResearchReference) => void;
  } = $props();
  const parts = $derived(
    journalParts(text).flatMap((part) =>
      part.resource || part.position || !onResearch ? [part] : researchTextParts(part.text, research),
    ) as { text: string; resource?: keyof typeof icons; position?: string; research?: ResearchReference }[],
  );
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

{#each parts as part}{#if part.research}<button
      class="journal-research-link"
      title={`View ${part.research.name}`}
      onclick={() => onResearch?.(part.research!)}>{part.text}</button
    >{:else if part.resource}{@const Icon = icons[part.resource]}<span class="inline-resource"
      ><Icon size={14} aria-hidden="true" />{part.text}</span
    >{:else if part.position && positions?.has(part.position) && onCoordinate}<button
      class="coordinate-link"
      title={`Highlight ${part.position} · Click to center on the map`}
      aria-label={`Locate ${part.position} on the map`}
      onmouseenter={() => onCoordinate?.(part.position!)}
      onmouseleave={(event) => {
        if (event.currentTarget !== document.activeElement) onCoordinate?.(null);
      }}
      onfocus={() => onCoordinate?.(part.position!)}
      onblur={() => onCoordinate?.(null)}
      onclick={() => onLocate?.(part.position!)}>{part.text}</button
    >{:else}{part.text}{/if}{/each}
