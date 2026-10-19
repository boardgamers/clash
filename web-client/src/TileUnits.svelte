<script lang="ts">
  import type { Player } from './types';
  import UnitIcon from './UnitIcon.svelte';
  let { players, position }: { players: Player[]; position: string } = $props();
  import { tileUnitStacks } from './tile-units';
  const stacks = $derived(tileUnitStacks(players, position));
</script>

{#each stacks as { player, groups }}
  <div class="tile-unit-stack" aria-label={`${player.civilization} units at ${position}`}>
    <small>{player.civilization}</small>
    {#each groups as group}
      {@const name = typeof group.type === 'object' ? group.type.Leader : group.name}
      <span title={`${group.count} ${name}`} aria-label={`${group.count} ${name}`}
        ><UnitIcon type={group.type} size={16} />{group.count}{#if group.aboard}<small>aboard</small
          >{/if}</span
      >
    {/each}
  </div>
{/each}
