<script lang="ts">
  let { selected, pending, onkeep, onraze, queued = false }: {
    selected: string | null; pending: boolean; queued?: boolean; onkeep: () => void; onraze: () => void;
  } = $props();
</script>
<div class="raze-actions">
  <button class="secondary" disabled={pending} onclick={onkeep}>{queued ? 'Queue keep all' : 'Keep all cities'}</button>
  <button class="primary" disabled={pending || !selected} onclick={onraze}>{queued ? 'Queue raze' : 'Raze'}{selected ? ` ${selected}` : ''}</button>
</div>
<style>
  .raze-actions { display: flex; gap: 8px; width: 100%; }
  button { flex: 1; min-height: 40px; border-radius: 4px; padding: 8px 12px; }
  .secondary { border: 1px solid var(--line); background: var(--paper2); }
</style>
