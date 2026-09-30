<script lang="ts">
  import { Skull } from 'lucide-svelte';
  import type { Game } from './types';
  import { positionXY } from './model';
  let {
    game,
    position,
    onHighlight,
  }: { game: Game; position: string; onHighlight: (position: string | null) => void } = $props();
  const pirateShips = $derived(
    game.players.find((p) => p.civilization === 'Pirates')?.units?.filter((u) => u.position === position) ??
      [],
  );
  const allied = $derived(
    game.players.some((p) => p.units?.some((u) => u.position === position && u.pirate)),
  );
  function adjacent(other: string) {
    const a = positionXY(position),
      b = positionXY(other);
    return Math.abs((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 - 3) < 0.001;
  }
  const blocked = $derived(
    game.map.tiles.filter(([p, t]) => t === 'Water' && (p === position || adjacent(p))).map(([p]) => p),
  );
</script>

{#if pirateShips.length}
  <section class="pirate-details" aria-label="Pirate effects">
    <h3><Skull size={17} />Pirate blockade</h3>
    <p>Blocks collection and Trade Routes here and on adjacent sea tiles.</p>
    <div class="pirate-coordinates" aria-label="Blocked sea tiles">
      {#each blocked as p}<button
          class="secondary"
          onmouseenter={() => onHighlight(p)}
          onmouseleave={() => onHighlight(null)}
          onfocus={() => onHighlight(p)}
          onblur={() => onHighlight(null)}>{p}</button
        >{/each}
    </div>
  </section>
{:else if allied}
  <section class="pirate-details" aria-label="Allied pirate effects">
    <h3><Skull size={17} />Pirate Allies</h3>
    <p>Carthage controls these ships while escorted. No pirate blockade.</p>
  </section>
{/if}
