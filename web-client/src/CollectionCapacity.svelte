<script lang="ts">
  import { Landmark, Smile, Meh, Frown } from 'lucide-svelte';
  let { size, mood, structures }: { size: number; mood: string; structures?: string[] } = $props();
  const Mood = $derived(mood === 'Happy' ? Smile : mood === 'Angry' ? Frown : Meh);
  const label = $derived(
    mood === 'Happy'
      ? `Size ${size} + Happy bonus 1 = ${size + 1} base collection capacity`
      : mood === 'Angry'
        ? `Angry: base collection capacity is 1, regardless of size`
        : `Neutral: base collection capacity equals city size ${size}`,
  );
</script>

<span class="collection-capacity" title={label} aria-label={label}>
  <span title={structures?.join(' + ')}><Landmark size={13} aria-hidden="true" />{size}</span>
  {#if mood === 'Happy'}+ <span><Mood size={14} aria-hidden="true" />1</span>
  {:else if mood === 'Angry'}→ <span><Mood size={14} aria-hidden="true" />1</span>
  {:else}<Mood size={14} aria-hidden="true" />{/if}
</span>
