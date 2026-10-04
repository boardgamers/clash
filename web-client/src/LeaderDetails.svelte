<script lang="ts">
  import { Crown, Sparkles, ChevronRight, ArrowRight } from 'lucide-svelte';
  import type { PlayerView, View, Move, Pile } from './types';
  import ResourceText from './ResourceText.svelte';
  let {
    leader,
    civilization,
    compact = false,
    actions = [],
    pending = false,
    onUse,
  }: {
    leader: Pick<NonNullable<PlayerView['leaders']>[number], 'name' | 'abilities'> & { position?: string };
    civilization?: string;
    compact?: boolean;
    actions?: NonNullable<View['specialActions']>;
    pending?: boolean;
    onUse?: (action: Move, payment?: Pile) => void;
  } = $props();
</script>

<details class="tile-leader" open={!compact}>
  <summary
    ><Crown size={17} /><strong>{leader.name}</strong>{#if civilization}<small>{civilization}</small
      >{/if}<ChevronRight size={14} /></summary
  >
  {#each leader.abilities as ability}
    {@const action = actions.find(
      (a) => a.name === ability.name && (!a.position || a.position === leader.position),
    )}
    <div class="tile-leader-ability">
      <strong><Sparkles size={13} />{ability.name}</strong>
      <p><ResourceText text={ability.description} /></p>
      {#if action && onUse}
        <button class="primary wide" disabled={pending} onclick={() => onUse?.(action.action, action.cost)}>
          Use {ability.name}<ArrowRight size={16} />
        </button>
      {/if}
    </div>
  {/each}
</details>
