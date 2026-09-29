<script lang="ts">
  import type { Player } from './types';
  import UnitIcon from './UnitIcon.svelte';
  let { players, position }: { players: Player[]; position: string } = $props();
  const stacks = $derived(
    players
      .map((player) => {
        const units = (player.units?.filter((unit) => unit.position === position) ?? []).flatMap((unit) => [
          unit,
          ...(unit.carried_units ?? []).map((carried) => ({ ...carried, position })),
        ]);
        const groups = new Map<string, { type: string | { Leader: string }; count: number }>();
        for (const unit of units) {
          const name = typeof unit.unit_type === 'string' ? unit.unit_type : unit.unit_type.Leader;
          const group = groups.get(name) ?? { type: unit.unit_type, count: 0 };
          group.count++;
          groups.set(name, group);
        }
        return { player, groups: [...groups] };
      })
      .filter((stack) => stack.groups.length),
  );
</script>

{#each stacks as { player, groups }}
  <div class="tile-unit-stack" aria-label={`${player.civilization} units at ${position}`}>
    <small>{player.civilization}</small>
    {#each groups as [name, group]}
      <span title={`${group.count} ${name}`} aria-label={`${group.count} ${name}`}
        ><UnitIcon type={group.type} size={16} />{group.count}</span
      >
    {/each}
  </div>
{/each}
