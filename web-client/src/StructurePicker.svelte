<script lang="ts" module>
  export interface StructureChoice {
    id: number;
    position: string;
    structure: string | { Building: string } | { Wonder: string };
  }
</script>

<script lang="ts">
  import { Check, Landmark } from 'lucide-svelte';
  import { buildingInfo } from './city';
  import { wonderIcons } from './wonder-icons';
  import { wonderName } from './wonder-names';
  let {
    choices,
    selected,
    position,
    onPosition,
    onSelect,
    onHighlight,
    pending = false,
    limit,
  }: {
    choices: StructureChoice[];
    selected: number[];
    position: string | null;
    onPosition: (position: string) => void;
    onSelect: (id: number) => void;
    onHighlight: (position: string | null) => void;
    pending?: boolean;
    limit: number;
  } = $props();
  const positions = $derived([...new Set(choices.map((c) => c.position))]);
  const current = $derived(position && positions.includes(position) ? position : positions[0]);
  const here = $derived(choices.filter((c) => c.position === current));
  const present = (structure: StructureChoice['structure']) =>
    typeof structure === 'string'
      ? { name: 'City center', icon: Landmark }
      : 'Building' in structure
        ? { name: structure.Building, icon: buildingInfo[structure.Building]?.icon ?? Landmark }
        : { name: wonderName(structure.Wonder), icon: wonderIcons[structure.Wonder] ?? Landmark };
</script>

<div class="structure-selection">
  <p class="movement-hint">Click a highlighted city or building on the map, or choose below.</p>
  {#if positions.length > 1}<div class="structure-cities" role="group" aria-label="Choose city">
      {#each positions as at}
        {@const count = choices.filter((c) => c.position === at && selected.includes(c.id)).length}
        <button
          class:selected={at === current}
          aria-pressed={at === current}
          disabled={pending}
          onclick={() => onPosition(at)}
          onmouseenter={() => onHighlight(at)}
          onmouseleave={() => onHighlight(null)}
          onfocus={() => onHighlight(at)}
          onblur={() => onHighlight(null)}
          ><Landmark size={14} />{at}{#if count}<b class="structure-city-count">{count}</b>{/if}</button
        >
      {/each}
    </div>{/if}
  <div class="decision-options structure-options" role="group" aria-label={`At ${current}`}>
    {#each here as choice (choice.id)}
      {@const { name, icon: Icon } = present(choice.structure)}
      <button
        class="decision-option"
        class:selected={selected.includes(choice.id)}
        aria-label={name}
        aria-pressed={selected.includes(choice.id)}
        disabled={pending || (!selected.includes(choice.id) && limit > 1 && selected.length >= limit)}
        onclick={() => onSelect(choice.id)}
        onmouseenter={() => onHighlight(choice.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(choice.position)}
        onblur={() => onHighlight(null)}
      >
        <Icon size={19} /><span class="decision-option-content"
          ><span class="decision-option-name">{name}</span></span
        >
        <span class="decision-selection" aria-hidden="true"
          >{#if selected.includes(choice.id)}<Check size={13} />{/if}</span
        >
      </button>
    {/each}
  </div>
</div>
