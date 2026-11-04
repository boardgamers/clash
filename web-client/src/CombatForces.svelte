<script lang="ts">
  import { combatForces } from './combat-forces';
  import type { Game, View } from './types';
  import UnitIcon from './UnitIcon.svelte';
  let { game, view }: { game: Game | null; view: View | null } = $props();
  const forces = $derived(combatForces(game));
  const summary = $derived(view?.combatSummary?.round === forces?.round ? view?.combatSummary : null);
  const sides = $derived([
    { label: 'Attacker', force: forces?.attacker, bonus: summary?.attacker },
    { label: 'Defender', force: forces?.defender, bonus: summary?.defender },
  ]);
</script>

{#if forces}
  <section class="combat-forces" aria-label="Current combat forces">
    <div class="force-heading">Battle · Round {forces.round}</div>
    <table
      class="combat-table"
      aria-label="Public combat bonuses before tactics"
      title="Public bonuses only · excludes tactics, dice-symbol effects and unresolved choices"
    >
      <thead
        ><tr
          ><td></td>{#each sides as side}<th scope="col">{side.label}</th>{/each}</tr
        ></thead
      >
      <tbody>
        <tr
          ><th scope="row">Units</th>{#each sides as side}<td
              ><div class="combat-units">
                {#if side.force}
                  {#each side.force.units as unit}
                    <span title={unit.label} aria-label={`${unit.count} ${unit.label}`}
                      ><UnitIcon type={unit.type} size={14} /><b>{unit.count}</b></span
                    >
                  {:else}None{/each}
                {:else}—{/if}
              </div></td
            >{/each}</tr
        >
        {#if summary}
          <tr
            ><th scope="row">Dice</th>{#each sides as side}<td
                ><b>{side.bonus?.dice}</b>{#if side.bonus?.extraDice}<span class="detail"
                    >Includes +{side.bonus.extraDice} bonus</span
                  >{/if}</td
              >{/each}</tr
          >
          <tr
            ><th scope="row">Value bonus</th>{#each sides as side}<td
                >{(side.bonus?.valueBonus ?? 0) > 0 ? '+' : ''}{side.bonus?.valueBonus}</td
              >{/each}</tr
          >
          {#if sides.some((s) => s.bonus?.effects.length || s.bonus?.inactive.length)}
            <tr
              ><th scope="row">Effects</th>{#each sides as side}<td>
                  {#each side.bonus?.effects ?? [] as effect}<span class="detail"
                      >{effect
                        .replace('fortress cancelled one hit', 'Fortress: cancels 1 hit (round 1)')
                        .replace('fortress added one extra die', 'Fortress: +1 die (round 1)')}</span
                    >{/each}
                  {#each side.bonus?.inactive ?? [] as effect}<span class="detail inactive">{effect}</span
                    >{/each}
                  {#if !side.bonus?.effects.length && !side.bonus?.inactive.length}—{/if}
                </td>{/each}</tr
            >
          {/if}
        {/if}
      </tbody>
    </table>
  </section>
{/if}

<style>
  .combat-forces {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.65rem;
    margin: 0.6rem 0;
  }
  .force-heading {
    color: var(--muted);
    font-size: 11px;
  }
  .combat-table {
    table-layout: fixed;
    width: 100%;
    margin: 0.35rem 0;
  }
  .combat-table th:first-child {
    width: 24%;
  }
  .detail {
    display: block;
    margin-top: 0.2rem;
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .inactive {
    color: var(--muted);
    font-style: italic;
  }
</style>
