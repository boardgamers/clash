<script lang="ts">
  import { Hourglass, Zap } from 'lucide-svelte';
  import ObjectiveArt from './ObjectiveArt.svelte';
  import ResourceText from './ResourceText.svelte';
  import type { View } from './types';
  let { objective }: { objective: View['objectiveCards'][number]['objectives'][number] } = $props();
</script>

<section class="objective-condition">
  <header>
    <ObjectiveArt name={objective.name} />
    <div>
      <h3>{objective.name}</h3>
      <span
        class="objective-timing"
        title={objective.timing === 'Status phase'
          ? 'Checked after everyone’s third turn of the age (the status phase). Meet the condition then to claim it.'
          : 'Offered when its condition is met during play.'}
        >{#if objective.timing === 'Status phase'}<Hourglass size={13} />End of age{:else}<Zap
            size={13}
          />During play{/if}</span
      >
    </div>
  </header>
  <p><ResourceText text={objective.description} /></p>
</section>
