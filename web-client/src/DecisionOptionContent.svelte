<script lang="ts">
  import { printedCardName } from './card-names';
  import { Hourglass, Zap, Layers, Swords } from 'lucide-svelte';
  import type { Decision } from './types';
  import ObjectiveArt from './ObjectiveArt.svelte';
  import ResourceText from './ResourceText.svelte';
  let { option, id }: { option: Decision['options'][number]; id: string } = $props();
</script>

<span {id} class="decision-option-content">
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
