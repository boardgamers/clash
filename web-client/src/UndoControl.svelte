<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Undo2, Redo2, ChevronDown } from 'lucide-svelte';
  let { canUndo, canRedo, blocked, submit }: { canUndo: boolean; canRedo: boolean; blocked: boolean; submit: (action: 'Undo' | 'Redo') => void } = $props();
  let open = $state(false), root = $state<HTMLDivElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let held = false;
  function cancel() { clearTimeout(timer); timer = undefined; }
  function start(event: PointerEvent) {
    held = false; cancel();
    if (!canRedo || blocked || event.button !== 0) return;
    timer = setTimeout(() => { held = true; open = true; }, 500);
  }
  function undo() {
    cancel();
    if (held) { held = false; return; }
    if (canUndo) { open = false; submit('Undo'); }
    else if (canRedo) open = !open;
  }
  $effect(() => { if (!canRedo || blocked) open = false; });
  onDestroy(cancel);
</script>
<svelte:window onpointerdown={event => { if (root && !root.contains(event.target as Node)) open = false; }} onkeydown={event => { if (event.key === 'Escape') open = false; }}/>
<div class="undo-control" bind:this={root}>
  <button title={canRedo ? 'Undo last action · Hold or use the menu for Redo' : 'Undo last action'}
    data-tutorial="undo" aria-label="Undo last action" disabled={blocked || (!canUndo && !canRedo)}
    onpointerdown={start} onpointerup={cancel} onpointerleave={cancel} onpointercancel={cancel}
    oncontextmenu={event => { if (canRedo && !blocked) { event.preventDefault(); held = true; open = true; } }}
    onkeydown={event => { if (canRedo && (event.key === 'ArrowDown' || (event.shiftKey && event.key === 'F10'))) { event.preventDefault(); open = true; } }}
    onclick={undo}><Undo2 size={19}/><span>Undo</span></button>
  {#if canRedo}
    <button class="undo-menu-toggle" title="Undo options" aria-label="Undo options" aria-expanded={open}
      disabled={blocked} onclick={() => open = !open}><ChevronDown size={10}/></button>
  {/if}
  {#if open && canRedo && !blocked}
    <div class="undo-options">
      <button title="Redo last undone action" aria-label="Redo last undone action" onclick={() => { open = false; submit('Redo'); }}><Redo2 size={17}/><span>Redo</span></button>
    </div>
  {/if}
</div>
<style>
  .undo-control { position: relative; min-width: 0; }
  .undo-control > button:first-child { width: 100%; }
  .undo-control .undo-menu-toggle { position: absolute; right: -3px; top: -4px; width: 12px; height: 12px; min-height: 12px; padding: 0; background: var(--paper2); border-radius: 4px; }
  .undo-menu-toggle :global(svg) { width: 10px !important; height: 10px !important; }
  .undo-options { position: absolute; right: 0; top: 100%; z-index: 30; padding: 5px; border: 1px solid var(--line); border-radius: 6px; background: var(--paper); box-shadow: 0 3px 12px #0003; }
  .undo-options button { display: flex !important; align-items: center; gap: 6px !important; padding: 8px 12px !important; white-space: nowrap; }
  .undo-options button span { display: inline !important; }
</style>
