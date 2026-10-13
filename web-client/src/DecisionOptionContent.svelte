<script lang="ts">
  import { printedCardName } from './card-names';
  import { Hourglass, Zap, Layers, Swords } from 'lucide-svelte';
  import type { Decision } from './types';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import ObjectiveArt from './ObjectiveArt.svelte';
  import ResourceText from './ResourceText.svelte';
  let { option, id }: { option: Decision['options'][number]; id: string } = $props();
</script>

<span {id} class="decision-option-content">
  {#if option.owner}<span class="decision-card-owner"><CivilizationEmblem civilization={option.owner.civilization} size={18}/>{option.owner.civilization}{option.owner.you ? ' · You' : ''}</span>{/if}
  {#if option.card?.kind === 'objective'}
    {#each option.card.objectives as objective, i}
      {#if i}<span class="decision-alternative">or</span>{/if}
      <span class="decision-card-face">
        <span class="decision-face-heading">
          <ObjectiveArt name={objective.name} />
          <span>
            <strong>{printedCardName(objective.name, 'objective')}</strong>
            <span class="decision-face-timing">
              {#if objective.timing === 'Status phase'}<Hourglass size={12} />End of age{:else}<Zap
                  size={12}
                />During play{/if}
            </span>
          </span>
        </span>
        <span class="decision-rule"><ResourceText text={objective.description} /></span>
        {#if objective.conditionMet}<span class="objective-ready">Condition met · End of age</span>{/if}
        {#if objective.progress?.length}<span class="objective-progress" aria-label="Owner’s current progress">
          {#each objective.progress as progress}<span class="objective-progress-row" class:met={progress.current >= progress.target}>
            <span>{progress.label}</span><strong>{progress.current}/{progress.target}</strong>
            <meter min="0" max={Math.max(1,progress.target)} value={Math.min(progress.current,progress.target)} aria-label={progress.label}></meter>
          </span>{/each}
        </span>{/if}
      </span>
    {/each}
  {:else if option.card?.kind === 'action'}
    <span class="decision-card-face">
      <span class="decision-use"><Layers size={13} />Action</span>
      <strong>{printedCardName(option.card.name)}</strong>
      <span class="decision-rule"><ResourceText text={option.card.description} /></span>
    </span>
    {#if option.card.tactics}
      <span class="decision-alternative">or</span>
      <span class="decision-card-face">
        <span class="decision-use"><Swords size={13} />Battle · Requires Tactics</span>
        <strong>{printedCardName(option.card.tactics.name)}</strong>
        <span class="decision-rule"><ResourceText text={option.card.tactics.description} /></span>
      </span>
    {/if}
  {:else}
    <span class="decision-option-name">{option.name}</span>
    {#if option.description}
      <span class="decision-rule"><ResourceText text={option.description} /></span>
    {/if}
  {/if}
</span>

<style>
  .decision-card-owner { display:flex; align-items:center; gap:.4rem; padding-bottom:.5rem; margin-bottom:.6rem; border-bottom:1px solid var(--line); color:var(--muted); font-size:.8rem; }
  .objective-progress { display:block; }
</style>
