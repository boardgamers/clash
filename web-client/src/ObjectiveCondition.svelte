<script lang="ts">
  import { printedCardName } from './card-names';
  import { Hourglass, Zap, CircleCheck } from 'lucide-svelte';
  import ObjectiveArt from './ObjectiveArt.svelte';
  import ResourceText from './ResourceText.svelte';
  import type { View } from './types';
  let { objective }: { objective: View['objectiveCards'][number]['objectives'][number] } = $props();
</script>

<section class="objective-condition">
  <header>
    <ObjectiveArt name={objective.name} />
    <div>
      <h3>{printedCardName(objective.name, 'objective')}</h3>
      <span
        class="objective-timing"
        title={objective.timing === 'Status phase'
          ? 'Objectives are checked before the free advance. Meet the condition at that check to claim it; an advance gained afterwards counts for the next age.'
          : 'Offered when its condition is met during play.'}
        >{#if objective.timing === 'Status phase'}<Hourglass size={13} />End of age{:else}<Zap
            size={13}
          />During play{/if}</span
      >
      {#if objective.timing === 'Status phase' && objective.conditionMet}
        <span class="objective-ready" title="Keep meeting this condition until the objective check.">
          <CircleCheck size={14} />Condition met · End of age {objective.scoringAge ?? ''}
        </span>
      {/if}
    </div>
  </header>
  <p><ResourceText text={objective.description} /></p>
  {#if objective.progress?.length}
    <div class="objective-progress" aria-label="Current progress">
      {#each objective.progress as progress}
        <div class="objective-progress-row" class:met={progress.current >= progress.target}>
          <span>{progress.label}</span>
          <strong
            >{#if progress.current >= progress.target}<CircleCheck
                size={13}
                aria-label="Target reached"
              />{/if}{progress.current}/{progress.target}</strong
          >
          <meter
            min="0"
            max={Math.max(1, progress.target)}
            value={Math.min(progress.current, progress.target)}
            aria-label={progress.label}>{progress.current}/{progress.target}</meter
          >
        </div>
      {/each}
    </div>
  {/if}
</section>
