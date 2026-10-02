<script lang="ts">
  import { Landmark, Zap } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { SettlerView } from './types';
  let { controller, settler }: { controller: Controller; settler: SettlerView } = $props();
  const session = $derived(controller.session);
  const reason = $derived(
    $session.view?.stopMovement
      ? 'Finish moving before founding a city.'
      : settler.foundReason === 'No actions left'
        ? 'No actions left · Founding a city needs 1 action.'
        : settler.foundReason,
  );
</script>

<div class="found-city-action">
  <button
    class="secondary"
    disabled={!settler.foundAction || $session.pending || !!$session.view?.stopMovement}
    title={reason ?? (settler.foundFree ? 'Founder · Free action' : 'Replaces this settler with a city')}
    onclick={() => settler.foundAction && controller.submit(settler.foundAction)}
  >
    <Landmark size={17} />Found city here
    <span class="settler-action-cost"
      >{#if settler.foundFree}Free{:else}<Zap size={13} />1{/if}</span
    >
  </button>
  {#if reason}<small>{reason}</small>{/if}
</div>
