<script lang="ts">
  import { Footprints, Swords, Ship, Crown, ChessKnight, PawPrint } from 'lucide-svelte';
  let { type, size = 16 }: { type: string | { Leader: string }; size?: number } = $props();
  const Icon = $derived(
    typeof type === 'object'
      ? Crown
      : type === 'Settler'
        ? Footprints
        : type === 'Ship'
          ? Ship
          : type === 'Cavalry'
            ? ChessKnight
            : type === 'Elephant'
              ? PawPrint
              : Swords,
  );
</script>

<Icon {size} aria-hidden="true" />
