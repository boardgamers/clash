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
    civilization?: string;
  }
</script>

<script lang="ts">
  import { Check, ChevronRight } from 'lucide-svelte';
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
    playerColors = [],
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
    playerColors?: string[];
    label?: string;
  } = $props();
  const positions = $derived([...new Set(choices.map((u) => u.position))]);
  const current = $derived(position && positions.includes(position) ? position : positions[0]);
  const here = $derived(choices.filter((u) => u.position === current));
</script>

<div class="unit-selection">
  <div class="unit-selection-heading">
    <span
      class="unit-selection-count"
      aria-label={`${selected.length}${limit !== undefined ? ` of ${limit}` : ''} units selected`}
    >
      {selected.length}{limit !== undefined ? ` / ${limit}` : ''} selected
    </span>
    {#if positions.length > 1}<button
        class="next-unit-group"
        disabled={pending}
        aria-label="Show next group of units"
        onclick={() => onPosition(positions[(positions.indexOf(current) + 1) % positions.length])}
        >Next group<ChevronRight size={14} /></button
      >{/if}
  </div>
  <div class="unit-choice-grid" role="group" aria-label={label}>
    {#each here as unit (unit.id)}
      <button
        class="unit-choice"
        class:selected={selected.includes(unit.id)}
        aria-label={`${unit.name}${unit.detail ? ` · ${unit.detail}` : ''}`}
        aria-pressed={selected.includes(unit.id)}
        disabled={pending ||
          (limit !== undefined && limit !== 1 && selected.length >= limit && !selected.includes(unit.id))}
        onclick={() => onSelect(unit.id)}
        onmouseenter={() => onHighlight(unit.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(unit.position)}
        onblur={() => onHighlight(null)}
      >
        <UnitPortrait
          type={unit.type}
          player={unit.player}
          pirate={unit.pirate}
          civilization={unit.civilization}
          {colorBlind}
          {playerColors}
        />
        <span class="unit-choice-check" aria-hidden="true"
          >{#if selected.includes(unit.id)}<Check size={14} />{/if}</span
        >
        <span class="unit-choice-caption"
          ><strong>{unit.name}</strong>
          {#if unit.detail}<small title={unit.detail}>{unit.detail.split(' · ')[0]}</small>{/if}
        </span>
      </button>
    {/each}
  </div>
</div>
