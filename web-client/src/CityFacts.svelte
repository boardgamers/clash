<script lang="ts">
  import { Landmark, Smile, Meh, Frown } from 'lucide-svelte';
  let { size, mood }: { size: number; mood: string } = $props();
  const Mood = $derived(mood === 'Happy' ? Smile : mood === 'Angry' ? Frown : Meh);
</script>

<span class="city-facts">
  <span title={`Size ${size}`} aria-label={`Size ${size}`}
    ><Landmark size={12} aria-hidden="true" />{size}</span
  >
  <span title={mood} aria-label={mood}><Mood size={15} aria-hidden="true" /></span>
</span>
