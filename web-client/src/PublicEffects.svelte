<script lang="ts">
  import { Layers, Target, Landmark, Trophy, X } from 'lucide-svelte';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const effect = $derived($session.publicEffects?.[0]);
  const player = $derived($session.game?.players.find((p) => p.id === effect?.player));
</script>

{#if effect}
  {#key effect.key}
    <aside
      class="public-effect"
      class:completed={effect.kind === 'completed'}
      class:still={$session.reducedMotion}
      role="status"
    >
      <span class="public-effect-card" aria-hidden="true"
        >{#if effect.kind === 'completed'}<Trophy size={25} />{:else if effect.kind === 'objective'}<Target
            size={25}
          />{:else if effect.kind === 'wonder'}<Landmark size={25} />{:else}<Layers size={25} />{/if}</span
      >
      <div>
        <small>{player?.civilization ?? 'Player'}</small><strong
          >{effect.kind === 'completed'
            ? 'Objective completed'
            : effect.kind === 'action'
              ? 'Action card drawn'
              : effect.kind === 'objective'
                ? 'Objective card drawn'
                : 'Wonder card drawn'}</strong
        >
        {#if effect.kind === 'completed'}<span>{effect.label}</span>{/if}
      </div>
      <button
        class="icon-button"
        aria-label="Dismiss notification"
        onclick={() => controller.dismissEffects()}><X size={15} /></button
      >
    </aside>
  {/key}
{/if}
