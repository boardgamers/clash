<script lang="ts">
  import { Play, Pause, ChevronLeft, ChevronRight, History, RotateCcw, X } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { frameDetails } from './playback';
  import ResourceText from './ResourceText.svelte';
  import BattlePlayback from './BattlePlayback.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const playback = $derived($session.playback);
  const frame = $derived(playback?.frame);
  const details = $derived(
    frameDetails(
      $session.game?.board_history?.frames[(playback?.index ?? 0) - 1],
      frame ?? null,
      $session.game,
      $session.view?.advances,
    ),
  );
  const atStart = $derived(!!playback && playback.index === playback.start);
  const atEnd = $derived(!!playback && playback.index >= playback.end);
  const recap = $derived(playback?.range !== 'all');
</script>

{#if playback}
  <section class="playback-bar floating-panel" aria-label="Board replay">
    {#if $session.battles?.length}<BattlePlayback {controller} />{/if}
    <div class="playback-description" aria-live="polite">
      <strong
        ><History size={16} />{playback.range === 'catch-up'
          ? 'Since your last turn'
          : playback.range === 'last-turn'
            ? 'Last turn'
            : 'Replay'}
        {#if playback.end > playback.start}<span class="playback-progress"
            >{atStart
              ? 'Start'
              : `Action ${playback.index - playback.start} of ${playback.end - playback.start}`}</span
          >{/if}
      </strong>
      <small
        >{#if atStart && recap}Use Next to step through, or Play to watch.{:else}<ResourceText
            text={details.caption}
            compactResources
          />{/if}</small
      >
    </div>
    {#if playback.end > playback.start}<div class="playback-transport">
        <button aria-label="Previous action" disabled={atStart} onclick={() => controller.stepPlayback(-1)}
          ><ChevronLeft size={16} />Back</button
        >
        <button
          class="playback-next"
          aria-label="Next action"
          disabled={atEnd}
          onclick={() => controller.stepPlayback(1)}>Next<ChevronRight size={16} /></button
        >
        {#if atEnd}
          <button aria-label="Replay this turn" onclick={() => controller.restartPlayback()}
            ><RotateCcw size={16} />Replay</button
          >
        {:else}
          <button
            aria-label={playback.playing ? 'Pause replay' : 'Play replay'}
            onclick={() => controller.togglePlayback()}
            >{#if playback.playing}<Pause size={16} />Pause{:else}<Play size={16} />Play{/if}</button
          >
        {/if}
      </div>{/if}
    {#if recap}<label class="playback-preference" title="Remember this preference for future opponent recaps">
        <input
          type="checkbox"
          checked={$session.replayAutoplay}
          onchange={(event) => controller.setReplayAutoplay(event.currentTarget.checked)}
        />Autoplay on return
      </label>{/if}
    <button class="playback-exit" onclick={() => controller.endPlayback()}>
      {atEnd ? 'Back to game' : recap ? 'Skip' : 'Done'}<X size={15} />
    </button>
  </section>
{/if}
