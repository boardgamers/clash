<script lang="ts">
  import {
    Landmark,
    GraduationCap,
    Target,
    Crown,
    ScrollText,
    Swords,
    Trophy,
    X,
    UserRound,
  } from 'lucide-svelte';
  import EventMarkers from './EventMarkers.svelte';
  import ResourceText from './ResourceText.svelte';
  import CivilizationAdvances from './CivilizationAdvances.svelte';
  import { researchPresentation } from './research';
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let player = $derived($session.view?.players.find((p) => p.index === $session.scorePlayer));
  let tab = $state<'score' | 'advances'>('score');
  let advances = $derived(
    [...(player?.advances ?? [])].sort((a, b) => a.order - b.order || a.name.localeCompare(b.name)),
  );
  let ordinaryAdvances = $derived(
    advances.filter((a) => !player?.civilizationAdvances.some((c) => c.id === a.id)),
  );
  let groups = $derived([...new Set(ordinaryAdvances.map((a) => a.group))]);
  const categories: Record<string, { icon: typeof Landmark; description: string }> = {
    'City pieces': {
      icon: Landmark,
      description: 'City centers and buildings you own, including buildings in other cities.',
    },
    Advances: { icon: GraduationCap, description: '½ point per advance, including civilization advances.' },
    Objectives: { icon: Target, description: 'Completed objectives and objective bonuses.' },
    Wonders: { icon: Crown, description: 'Points for building and owning wonders.' },
    Events: { icon: ScrollText, description: 'Points awarded by events and other abilities.' },
    'Captured Leaders': { icon: Swords, description: 'Points for enemy leaders you have captured.' },
  };
  const close = () => controller.patch({ scorePlayer: null });
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

{#if player}
  <dialog
    class="field-guide score-dialog"
    aria-labelledby="score-title"
    use:show
    onclose={close}
    onclick={(e) => {
      if (e.target === e.currentTarget) close();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') close();
    }}
  >
    <button class="close-guide icon-button" aria-label="Close civilization details" onclick={close}
      ><X size={20} /></button
    >
    <span class="card-eyebrow">Civilization</span>
    <h2 id="score-title">{player.civilization} <EventMarkers remaining={player.eventTokens} /></h2>
    <nav class="civilization-tabs" aria-label="Civilization details">
      <button class:active={tab === 'score'} aria-pressed={tab === 'score'} onclick={() => (tab = 'score')}
        ><Trophy size={16} /> Score</button
      >
      <button
        class:active={tab === 'advances'}
        aria-pressed={tab === 'advances'}
        onclick={() => (tab = 'advances')}
        ><GraduationCap size={16} /> Advances <span>{advances.length}</span></button
      >
    </nav>
    {#if tab === 'score'}
      <div class="score-summary">
        <span><Trophy size={22} /> Total</span><strong>{player.score}<small> VP</small></strong>
      </div>
      <div class="score-breakdown">
        {#each player.scoreParts as part}
          {@const category = categories[part.name]}
          {@const Icon = category?.icon ?? Trophy}
          <div class="score-row" class:zero={part.points === 0}>
            <Icon size={19} />
            <div>
              <strong>{part.name}</strong>
              <p>{category?.description}</p>
            </div>
            <b>{part.points}</b>
          </div>
        {/each}
      </div>
    {:else}
      {#if player.civilizationAdvances.length}<CivilizationAdvances
          civilization={player.civilization}
          advances={player.civilizationAdvances}
        />{/if}
      <div class="public-advances">
        {#each groups as group}
          <section aria-label={group}>
            <h3>{group}</h3>
            {#each ordinaryAdvances.filter((a) => a.group === group) as advance}
              {@const presentation = researchPresentation(advance)}
              {@const Icon = presentation.icon}
              <article title={advance.description}>
                <span class="advance-pictogram"><Icon size={21} /></span>
                <div>
                  <strong>{advance.name}</strong>{#if advance.borrowed}<small>Great Library</small>{/if}
                  <p><ResourceText text={presentation.summary} /></p>
                </div>
              </article>
            {/each}
          </section>
        {:else}<p>No advances researched.</p>{/each}
      </div>
    {/if}
    <button class="score-profile secondary" onclick={() => controller.commands.openPlayer(player!.index)}
      ><UserRound size={15} /> {player.name} · Player profile</button
    >
  </dialog>
{/if}
