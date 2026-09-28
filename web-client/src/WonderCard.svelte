<script lang="ts">
  import {
    Landmark,
    Mountain,
    Trees,
    LibraryBig,
    TowerControl,
    Castle,
    Theater,
    Crown,
    GraduationCap,
    Check,
    Trophy,
  } from 'lucide-svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import type { WonderCard } from './types';
  let { card }: { card: WonderCard } = $props();
  const icons: Record<string, typeof Landmark> = {
    Pyramids: Mountain,
    GreatGardens: Trees,
    GreatLibrary: LibraryBig,
    GreatLighthouse: TowerControl,
    GreatWall: Castle,
    Colosseum: Theater,
    GreatStatue: Crown,
  };
  let Icon = $derived(icons[card.id] ?? Landmark);
</script>

<article class="wonder-card" aria-label={`Wonder card: ${card.name}`}>
  <div class="wonder-heading">
    <span class="wonder-emblem" aria-hidden="true"><Icon size={34} strokeWidth={1.3} /></span>
    <div>
      <span class="card-eyebrow">Wonder</span>
      <h3>{card.name}</h3>
    </div>
  </div>
  <p class="wonder-effect">{card.description}</p>
  <div class="wonder-requirement">
    <GraduationCap size={16} /><span>Requires <strong>{card.requiredAdvance}</strong></span
    >{#if card.requiredAdvanceOwned}<Check size={15} aria-label="Researched" />{/if}
  </div>
  <div class="wonder-cost"><span>Base construction cost</span><ResourceAmount pile={card.cost} /></div>
  <div class="wonder-points">
    <Trophy size={16} /><span
      ><strong>{card.builtPoints}</strong> VP for building · <strong>{card.ownedPoints}</strong> VP for owning</span
    >
  </div>
</article>
