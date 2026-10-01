<script lang="ts">
  import type { UnitView } from './types';
  import { playerColor } from './types';
  import { unitPortrait } from './unit-portrait';
  import UnitIcon from './UnitIcon.svelte';
  let {
    type,
    player,
    colorBlind = false,
    pirate = false,
    civilization,
  }: {
    type: UnitView['type'];
    player: number;
    colorBlind?: boolean;
    pirate?: boolean;
    civilization?: string;
  } = $props();
  let src = $state('');
  $effect(() => {
    try {
      src = unitPortrait(type, playerColor(player, colorBlind), pirate, civilization);
    } catch {
      src = '';
    }
  });
</script>

<span class="unit-portrait" aria-hidden="true">
  {#if src}<img {src} alt="" draggable="false" />{:else}<UnitIcon {type} size={38} />{/if}
</span>
