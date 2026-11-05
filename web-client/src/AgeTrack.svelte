<script lang="ts">
  import { Dice5, Landmark, Info, X } from 'lucide-svelte';
  import type { Game } from './types';
  import { ageCount, ageLabel, lengthRule, lengthName, variableEndThreshold } from './game-length';
  import { currentLocale, translateText } from './localization';
  let {
    game,
    age,
    round,
    roundHint = '',
  }: { game: Game | null; age: number; round: number; roundHint?: string | null } = $props();
  const t = $derived((text: string) => translateText(text, $currentLocale));
  let selected = $state<number | null>(null);
  let root: HTMLDivElement;
  let opener: HTMLButtonElement | undefined;
  const mode = $derived(game?.options?.length ?? 'Standard');
  const variable = $derived(mode === 'RollToEnd' || mode === 'WonderfulEnding');
  const threshold = $derived(variableEndThreshold(age, round));
  const triggered = $derived(game?.variable_end === 'WonderfulEnding' && game?.state !== 'Finished');
  function toggle(value: number, event: MouseEvent) {
    opener = event.currentTarget as HTMLButtonElement;
    selected = selected === value ? null : value;
  }
  function close() {
    selected = null;
    opener?.focus();
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && selected !== null) {
      event.stopPropagation();
      close();
    }
  }}
  onclick={(event) => {
    if (selected !== null && !root?.contains(event.target as Node)) selected = null;
  }}
/>
<div bind:this={root} class="age-track" role="group" aria-label={t('Game length')}>
  <div class="age-steps">
    {#each Array.from({ length: ageCount(game) }, (_, i) => i + 1) as step}
      <button
        type="button"
        class="age-step"
        class:current={step === age}
        class:past={step < age}
        aria-current={step === age ? 'step' : undefined}
        aria-expanded={selected === step}
        aria-controls="age-length-details"
        aria-label={`${t('Game length')} · ${t(lengthName(game))} · ${ageLabel(step)}`}
        title={`${t(lengthName(game))} · ${variable && step >= 6 ? t(lengthRule(game)) : t('Game length')}`}
        onclick={(event) => toggle(step, event)}
      >
        {ageLabel(step)}
        {#if variable && step >= 6}
          <span class="age-end-icon" class:inactive={step === age && threshold === null} aria-hidden="true">
            {#if mode === 'RollToEnd'}<Dice5 size={14} />{:else}<Landmark size={12} />{/if}
          </span>
        {:else if step === ageCount(game)}<span class="age-end-icon" aria-hidden="true"
            ><Info size={11} /></span
          >{/if}
      </button>
    {/each}
  </div>
  <small class="round-label">
    <span
      >{round > 3 ? t('End of age') : `${t('Round')} ${round}/3`}{#if roundHint}
        · {roundHint}{/if}</span
    >
    {#if mode === 'WonderfulEnding' && (threshold !== null || triggered)}
      <button
        class="age-phase"
        onclick={(event) => toggle(age, event)}
        aria-expanded={selected !== null}
        aria-controls="age-length-details"
        ><Landmark size={12} />{t(triggered ? 'Ends this round' : 'Wonder to end')}</button
      >
    {:else if mode === 'RollToEnd' && threshold !== null}
      <button
        class="age-phase"
        onclick={(event) => toggle(age, event)}
        aria-label={t('Roll to End')}
        title={t(lengthRule(game))}
        aria-expanded={selected !== null}
        aria-controls="age-length-details"
        ><Dice5 size={14} />{threshold === 6 ? '6' : `${threshold}–6`}</button
      >
    {/if}
  </small>
  {#if selected !== null}
    <section id="age-length-details" class="age-length-details" aria-label={t('Game length')}>
      <button class="age-details-close" onclick={close} aria-label={t('Close dialog')}><X size={16} /></button
      >
      <strong>{t(lengthName(game))} · {t('Age')} {ageLabel(selected)}</strong>
      <p>{t(lengthRule(game))}</p>
      {#if mode === 'RollToEnd'}
        <table class="age-end-thresholds">
          <thead
            ><tr
              ><th scope="col">{t('Age')}</th><th scope="col">{t('Round')}</th><th scope="col"
                >{t('End on')}</th
              ></tr
            ></thead
          >
          <tbody>
            {#each [[6, 2, 6], [6, 3, 5], [7, 1, 4], [7, 2, 3], [7, 3, 2]] as [endAge, endRound, endThreshold]}
              <tr
                class:current={age === endAge && round === endRound}
                aria-current={age === endAge && round === endRound ? 'step' : undefined}
              >
                <td>{ageLabel(endAge)}</td><td>{endRound}</td><td
                  >{endThreshold === 6 ? '6' : `${endThreshold}–6`}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      {#if triggered}<p class="age-ending">
          {t('Wonderful Ending: finish this round, then complete objectives')}
        </p>{/if}
    </section>
  {/if}
</div>
