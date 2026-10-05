<script lang="ts">
  import { Wheat, Trees, Mountain, Lightbulb, Coins, Smile, Drama, Link } from 'lucide-svelte';
  import type { CardReference } from './card-reference';
  import { journalParts } from './model';
  import { namedTextParts, researchTextParts, type ResearchReference } from './research-links';
  let {
    text,
    compactResources = false,
    namedResources = false,
    positions,
    onCoordinate,
    onLocate,
    research = [],
    onResearch,
    cards = [],
    onCard,
    objectives = [],
    onObjective,
  }: {
    text: string;
    compactResources?: boolean;
    namedResources?: boolean;
    positions?: Set<string>;
    onCoordinate?: (position: string | null) => void;
    onLocate?: (position: string) => void;
    research?: ResearchReference[];
    onResearch?: (reference: ResearchReference) => void;
    cards?: CardReference[];
    onCard?: (reference: CardReference) => void;
    objectives?: { name: string; player: number }[];
    onObjective?: (reference: { name: string; player: number }) => void;
  } = $props();
  const parts = $derived(
    journalParts(text, namedResources).flatMap((part) => {
      if (part.resource || part.position) return [part];
      return namedTextParts(part.text, onObjective ? objectives : []).flatMap(({ text, reference }) =>
        reference
          ? [{ text, objective: reference }]
          : namedTextParts(text, onCard ? cards : []).flatMap(({ text, reference }) =>
              reference
                ? [{ text, card: reference }]
                : onResearch
                  ? researchTextParts(text, research)
                  : [{ text }],
            ),
      );
    }) as {
      text: string;
      resource?: keyof typeof icons;
      position?: string;
      research?: ResearchReference;
      card?: CardReference;
      objective?: { name: string; player: number };
    }[],
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

{#each parts as part}{#if part.card}<button
      class="journal-research-link"
      title={`View ${part.card.name} rules`}
      onclick={() => onCard?.(part.card!)}>{part.text}</button
    >
  {:else if part.objective}<button
      class="journal-research-link"
      title={`View completed objective ${part.objective.name}`}
      onclick={() => onObjective?.(part.objective!)}>{part.text}</button
    >{:else if part.research}<button
      class="journal-research-link"
      title={`View ${part.research.name}`}
      onclick={() => onResearch?.(part.research!)}>{part.text}</button
    >{:else if part.resource}{@const Icon = icons[part.resource]}<span
      class="inline-resource"
      role={compactResources ? 'img' : undefined}
      aria-label={compactResources ? part.text : undefined}
      title={compactResources ? part.text : undefined}
      ><Icon size={14} aria-hidden="true" />{compactResources
        ? part.text.split(/\s+(?=[a-z])/i)[0]
        : part.text}</span
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
