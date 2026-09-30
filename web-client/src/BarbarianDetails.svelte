<script lang="ts">
  import { Tent, Footprints, Users, WheatOff } from 'lucide-svelte';
  import type { Game } from './types';
  let { game, position }: { game: Game; position: string } = $props();
  const barbarians = $derived(game.players.find((player) => player.civilization === 'Barbarians'));
  const city = $derived(barbarians?.cities?.some((city) => city.position === position));
  const units = $derived(barbarians?.units?.filter((unit) => unit.position === position) ?? []);
</script>

{#if city || units.length}
  <section class="barbarian-details" aria-label="Barbarian effects">
    <header>
      <h3><Tent size={17} />Barbarians</h3>
      <span title="Maximum 4 army units per tile">Army {units.length}/4</span>
    </header>
    {#if city}<p>
        <Users size={16} /><span
          >After a “Barbarians move” event, cities within 2 land spaces of the triggering player’s cities gain
          1 infantry, up to 4 units.</span
        >
      </p>{/if}
    <p>
      <Footprints size={16} /><span
        >“Barbarians move” events move nearby armies 1 land tile toward the triggering player’s cities.</span
      >
    </p>
    <p><WheatOff size={16} /><span>Blocks resource collection on this tile.</span></p>
  </section>
{/if}
