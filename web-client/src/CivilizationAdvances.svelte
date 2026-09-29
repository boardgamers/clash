<script lang="ts">
  import { Check, LockKeyhole, Pause } from 'lucide-svelte';
  import type { CivilizationAdvance } from './types';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import ResourceText from './ResourceText.svelte';
  import { researchPresentation } from './research';
  let {
    civilization,
    advances,
    onPrerequisite,
  }: {
    civilization: string;
    advances: CivilizationAdvance[];
    onPrerequisite?: (id: string) => void;
  } = $props();
</script>

<section class="civilization-advances" aria-label={`${civilization} advances`}>
  <h3><CivilizationEmblem {civilization} size={18} />{civilization} advances</h3>
  <p
    class="civilization-unlock-note"
    title="Unlock automatically with the required research. No extra action, resources or event marker."
  >
    Automatic · ½ VP each
  </p>
  <div class="civilization-advance-grid">
    {#each advances as advance}
      {@const Icon = researchPresentation(advance).icon}
      <article class="civilization-advance" class:owned={advance.owned} id={`civilization-${advance.id}`}>
        <div class="civilization-advance-title">
          <span class="advance-pictogram"><Icon size={21} /></span>
          <strong>{advance.name}</strong>
          <span class="civilization-advance-status">
            {#if advance.active}<Check size={14} />Unlocked
            {:else if advance.owned}<Pause size={14} />Inactive
            {:else}<LockKeyhole size={12} />Locked{/if}
          </span>
        </div>
        <p class="civilization-advance-effect"><ResourceText text={advance.description} /></p>
        <div class="civilization-prerequisites" title={`Unlocks automatically with ${advance.requirement}`}>
          <span>{advance.owned ? 'From' : 'Unlock with'}</span>
          {#each advance.prerequisites as required, index}
            {#if index > 0}<span>or</span>{/if}
            {#if onPrerequisite}
              <button onclick={() => onPrerequisite?.(required.id)}>{required.name}</button>
            {:else}<strong>{required.name}</strong>{/if}
          {/each}
        </div>
      </article>
    {/each}
  </div>
</section>
