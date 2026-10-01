<script lang="ts" module>
  import type { UnitView } from './types';
  export interface UnitChoice {
    id: number;
    type: UnitView['type'];
    player: number;
    position: string;
    name: string;
    detail?: string;
    pirate?: boolean;
  }
</script>

<script lang="ts">
  import { Check, MapPin } from 'lucide-svelte';
  import UnitPortrait from './UnitPortrait.svelte';
  let {
    choices,
    selected,
    position,
    onPosition,
    onSelect,
    onHighlight,
    pending = false,
    limit,
    colorBlind = false,
    label = 'Choose units',
  }: {
    choices: UnitChoice[];
    selected: number[];
    position: string | null;
    onPosition: (position: string) => void;
    onSelect: (id: number) => void;
    onHighlight: (position: string | null) => void;
    pending?: boolean;
    limit?: number;
    colorBlind?: boolean;
    label?: string;
  } = $props();
  const positions = $derived([...new Set(choices.map((u) => u.position))]);
  const current = $derived(position && positions.includes(position) ? position : positions[0]);
  const here = $derived(choices.filter((u) => u.position === current));
</script>

<div class="unit-selection">
  <div class="unit-selection-heading">
    <div class="unit-location-bar" role="group" aria-label="Unit locations">
      {#each positions as location}
        {@const count = choices.filter((u) => u.position === location && selected.includes(u.id)).length}
        <button
          class:selected={current === location}
          aria-pressed={current === location}
          aria-label={`Units at ${location}`}
          disabled={pending}
          onmouseenter={() => onHighlight(location)}
          onmouseleave={() => onHighlight(null)}
          onfocus={() => onHighlight(location)}
          onblur={() => onHighlight(null)}
          onclick={() => onPosition(location)}
        >
          <MapPin size={14} />{location}{#if count}<span class="location-selected"
              ><Check size={12} />{count}</span
            >{/if}
        </button>
      {/each}
    </div>
    <strong
      class="unit-selection-count"
      aria-label={`${selected.length}${limit !== undefined ? ` of ${limit}` : ''} units selected`}
      >{selected.length}{limit !== undefined ? ` / ${limit}` : ''} selected</strong
    >
  </div>
  <div class="unit-choice-grid" role="group" aria-label={`${label} at ${current}`}>
    {#each here as unit (unit.id)}
      <button
        class="unit-choice"
        class:selected={selected.includes(unit.id)}
        aria-label={`${unit.name} at ${unit.position}${unit.detail ? ` · ${unit.detail}` : ''}`}
        aria-pressed={selected.includes(unit.id)}
        disabled={pending ||
          (limit !== undefined && limit !== 1 && selected.length >= limit && !selected.includes(unit.id))}
        onclick={() => onSelect(unit.id)}
        onmouseenter={() => onHighlight(unit.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(unit.position)}
        onblur={() => onHighlight(null)}
      >
        <UnitPortrait type={unit.type} player={unit.player} pirate={unit.pirate} {colorBlind} />
        <span class="unit-choice-check" aria-hidden="true"
          >{#if selected.includes(unit.id)}<Check size={14} />{/if}</span
        >
        <strong>{unit.name}</strong>
        {#if unit.detail}<small>{unit.detail}</small>{/if}
      </button>
    {/each}
  </div>
</div>
