<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
  import type { CombatRound } from './combat-journal';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import UnitIcon from './UnitIcon.svelte';
  let { combat }: { combat: CombatRound } = $props();
  const sides = $derived([combat.attacker, combat.defender]);
</script>

<table class="combat-table" aria-label={`Combat round ${combat.round ?? ''}`}>
  <thead
    ><tr
      ><td></td>{#each sides as side, i}<th scope="col">
          <span class="combat-role">{i === 0 ? 'Attacker' : 'Defender'}</span>
          <span class="combat-civilization"
            >{#if side.civilization}<CivilizationEmblem
                civilization={side.civilization}
                size={18}
              />{/if}{side.civilization ?? '—'}</span
          >
        </th>{/each}</tr
    ></thead
  >
  <tbody>
    <tr
      ><th scope="row">Units</th>{#each sides as side}<td
          ><div class="combat-units">
            {#if side.units?.length}{#each side.units as unit}<span title={unit.label} aria-label={unit.label}
                  ><UnitIcon type={unit.type} size={14} /><b>{unit.count}</b></span
                >{/each}
            {:else}{side.units ? 'None' : '—'}{/if}
          </div></td
        >{/each}</tr
    >
    {#if sides.some((s) => s.tactics)}<tr
        ><th scope="row">Tactics</th>{#each sides as side}<td>{side.tactics ?? '—'}</td>{/each}</tr
      >{/if}
    <tr
      ><th scope="row">Dice</th>{#each sides as side}<td
          ><div class="combat-dice">
            {#each side.dice ?? [] as die}<span
                class="combat-die"
                class:rerolled={die.effect === 're-roll'}
                title={die.effect ? `${die.value} · ${die.symbol}: ${die.effect}` : `${die.value}`}
              >
                <b>{die.value}</b>{#if die.effect === 're-roll'}<RotateCcw size={10} />{/if}
              </span>{:else}{side.dice ? 'None' : '—'}{/each}
          </div>
          {#each side.dice?.filter((d) => d.effect && d.effect !== 're-roll') ?? [] as die}
            <span class="combat-die-effect"
              ><UnitIcon type={die.symbol === 'Leader' ? { Leader: 'Leader' } : die.symbol!} size={12} />
              {#if die.effect === '-1 hits, no combat value'}<span>Blocks 1 hit<br />0 value</span
                >{:else}{die.effect?.replace('combat value', 'value')}{/if}
            </span>
          {/each}
        </td>{/each}</tr
    >
    {#if sides.some((s) => s.modifiers.length)}<tr
        ><th scope="row">Effects</th>{#each sides as side}<td>
            {#each side.modifiers as modifier}<span class="combat-modifier">{modifier}</span>{:else}—{/each}
          </td>{/each}</tr
      >{/if}
    <tr
      ><th scope="row">Value</th>{#each sides as side}<td class="combat-value">{side.value ?? '—'}</td
        >{/each}</tr
    >
    <tr class="combat-hits"
      ><th scope="row">Hits dealt</th>{#each sides as side}<td><b>{side.hits ?? '—'}</b></td>{/each}</tr
    >
  </tbody>
</table>
{#if combat.result}<div class="combat-result">{combat.result}</div>{/if}
