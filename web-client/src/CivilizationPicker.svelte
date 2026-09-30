<script lang="ts">
  import { ArrowRight, Crown, GraduationCap } from 'lucide-svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import ResourceText from './ResourceText.svelte';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const civilizations = $derived($session.view?.civilizations ?? []);
  let selected = $state('');
  let tab = $state<'advances' | 'leaders'>('advances');
  const civilization = $derived(civilizations.find((c) => c.name === selected) ?? civilizations[0]);
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide civilization-picker"
  aria-labelledby="civilization-title"
  use:show
  oncancel={(e) => e.preventDefault()}
>
  <h2 id="civilization-title">Choose civilization</h2>
  <div class="faction-choices" role="group" aria-label="Available civilizations">
    {#each civilizations as option}
      <button
        class:selected={civilization?.name === option.name}
        aria-pressed={civilization?.name === option.name}
        onclick={() => (selected = option.name)}
      >
        <CivilizationEmblem civilization={option.name} />{option.name}
      </button>
    {/each}
  </div>
  {#if civilization}
    <div class="faction-tabs" role="tablist" aria-label={`${civilization.name} details`}>
      <button
        role="tab"
        aria-selected={tab === 'advances'}
        class:selected={tab === 'advances'}
        onclick={() => (tab = 'advances')}><GraduationCap size={17} />Advances</button
      >
      <button
        role="tab"
        aria-selected={tab === 'leaders'}
        class:selected={tab === 'leaders'}
        onclick={() => (tab = 'leaders')}><Crown size={17} />Leaders</button
      >
    </div>
    <div
      class="faction-details"
      role="tabpanel"
      aria-label={tab === 'advances' ? 'Civilization advances' : 'Leaders'}
    >
      {#if tab === 'advances'}
        {#each civilization.advances as advance}
          <article>
            <h3>{advance.name}</h3>
            <small>Unlocks with {advance.requirement}</small>
            <p><ResourceText text={advance.description} /></p>
          </article>
        {/each}
      {:else}
        {#each civilization.leaders as leader}
          <article>
            <h3><Crown size={17} />{leader.name}</h3>
            {#each leader.abilities as ability}<p>
                <strong>{ability.name}</strong><br /><ResourceText text={ability.description} />
              </p>{/each}
          </article>
        {/each}
      {/if}
    </div>
    <footer>
      <button
        class="primary wide"
        disabled={$session.pending}
        onclick={() => controller.submit(civilization.action)}
        >Play as {civilization.name}<ArrowRight size={17} /></button
      >
    </footer>
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</dialog>
