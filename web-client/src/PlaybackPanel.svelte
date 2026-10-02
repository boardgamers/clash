<script lang="ts">
  import { Play, Pause, ChevronLeft, ChevronRight, History, X } from 'lucide-svelte';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const playback = $derived($session.playback);
  const frame = $derived(playback?.frame);
  const actor = $derived(frame?.players.find((p) => p.id === frame.actor)?.civilization);
</script>

{#if playback}
  <section class="playback-bar floating-panel" class:automatic={playback.automatic} aria-label="Board replay">
    <div class="playback-description">
      <strong><History size={16} />{playback.automatic ? 'Since your last turn' : 'Replay'}</strong>
      <small
        >{frame
          ? `${actor ? actor + ' · ' : ''}${frame.title}`
          : 'No recorded positions yet. New actions will appear here.'}</small
      >
    </div>
    {#if playback.total > 1}<div class="playback-transport">
        <button
          class="icon-button"
          aria-label="Previous action"
          disabled={playback.index === 0}
          onclick={() => controller.stepPlayback(-1)}><ChevronLeft size={18} /></button
        >
        <button
          class="icon-button"
          aria-label={playback.playing ? 'Pause replay' : 'Play replay'}
          onclick={() => controller.togglePlayback()}
          >{#if playback.playing}<Pause size={18} />{:else}<Play size={18} />{/if}</button
        >
        <button
          class="icon-button"
          aria-label="Next action"
          disabled={playback.index >= playback.total - 1}
          onclick={() => controller.stepPlayback(1)}><ChevronRight size={18} /></button
        >
        <small>{playback.index + 1}/{playback.total}</small>
      </div>{/if}
    <button class="playback-exit" onclick={() => controller.endPlayback()}
      >{playback.automatic ? 'Skip' : 'Done'}<X size={15} /></button
    >
  </section>
{/if}
