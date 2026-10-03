<script lang="ts">
  import { CircleCheck, RotateCw, TriangleAlert, Smile, Meh, Frown, ArrowRight, Ban } from 'lucide-svelte';
  import type { CityView } from './types';
  let { city, warning = false }: { city: CityView; warning?: boolean } = $props();
  const moods: Record<string, typeof Smile> = { Happy: Smile, Neutral: Meh, Angry: Frown };
  let Before = $derived(moods[city.mood] ?? Smile);
  let After = $derived(moods[city.activationMood] ?? Smile);
  let decreases = $derived(city.activationMood !== city.mood);
</script>

<div
  class="activation-status"
  class:activation-warning={warning && (decreases || city.mood === 'Angry')}
  class:activation-blocked={!city.canActivate}
>
  <div class="activation-heading">
    {#if !city.canActivate}<Ban
        size={17}
      />{:else if warning && (decreases || city.mood === 'Angry')}<TriangleAlert
        size={17}
      />{:else if city.activations}<RotateCw size={16} />{:else}<CircleCheck size={16} />{/if}
    <strong
      >{!city.canActivate
        ? 'City cannot activate again'
        : city.activations
          ? `Activated ${city.activations} ${city.activations === 1 ? 'time' : 'times'} this turn`
          : 'Not activated this turn'}</strong
    >
  </div>
  {#if !city.canActivate}
    <p>Already activated while angry. Wait until next turn or improve its happiness.</p>
  {:else if warning && decreases}
    <div class="activation-mood">
      <span class="city-mood" data-mood={city.mood.toLowerCase()}><Before size={17} /></span
      >{city.mood}<ArrowRight size={15} /><span
        class="city-mood"
        data-mood={city.activationMood.toLowerCase()}><After size={17} /></span
      ><strong>{city.activationMood}</strong>
    </div>
    <p>
      This activation lowers mood. Afterwards, this city’s base collection and recruitment capacity is {city.activationCapacity}.
    </p>
  {:else if warning && city.mood === 'Angry'}
    <p>Activating while angry blocks further activations until next turn or until happiness improves.</p>
  {:else if !city.activations}
    <p>The first activation keeps its mood. Further activations lower it.</p>
  {/if}
</div>
