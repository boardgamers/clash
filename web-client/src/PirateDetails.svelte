<script lang="ts">
  import { Skull, Waves, Coins, Swords } from 'lucide-svelte';
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
  const cities = $derived(
    game.players
      .filter((p) => !['Pirates', 'Barbarians'].includes(p.civilization))
      .flatMap((p) =>
        (p.cities ?? [])
          .filter((c) => adjacent(c.position))
          .map((c) => ({ ...c, civilization: p.civilization })),
      ),
  );
</script>

{#if pirateShips.length}
  <section class="pirate-details" aria-label="Pirate effects">
    <h3><Skull size={17} />Pirates</h3>
    <div>
      <h4><Waves size={15} />Blockade · Ongoing</h4>
      <p>Collection and Trade Routes are blocked on this sea tile and adjacent sea tiles.</p>
      <div class="pirate-coordinates" aria-label="Blocked sea tiles">
        {#each blocked as p}<button
            class="secondary"
            onmouseenter={() => onHighlight(p)}
            onmouseleave={() => onHighlight(null)}
            onfocus={() => onHighlight(p)}
            onblur={() => onHighlight(null)}>{p}</button
          >{/each}
      </div>
    </div>
    <div>
      <h4><Coins size={15} />Raid · When a pirate event occurs</h4>
      <p>
        Each player with an adjacent city pays 1 resource, mood token or culture token in total. If unable to
        pay, lower one adjacent city's mood.
      </p>
      {#if cities.length}<div class="pirate-coordinates" aria-label="Cities exposed to pirate raids">
          {#each cities as city}<button
              class="secondary"
              onmouseenter={() => onHighlight(city.position)}
              onmouseleave={() => onHighlight(null)}
              onfocus={() => onHighlight(city.position)}
              onblur={() => onHighlight(null)}>{city.civilization} · {city.position}</button
            >{/each}
        </div>{/if}
    </div>
    <div>
      <h4><Swords size={15} />Naval battle</h4>
      <p>
        Moving or recruiting ships onto this tile starts a battle. Pirate Allies allows Carthage to enter
        peacefully.
      </p>
    </div>
  </section>
{:else if allied}
  <section class="pirate-details" aria-label="Allied pirate effects">
    <h3><Skull size={17} />Pirate Allies</h3>
    <p>
      These ships belong to Carthage while escorted by a Carthaginian unit. They can carry troops and trade,
      and cause no pirate blockade or raids while allied.
    </p>
  </section>
{/if}
