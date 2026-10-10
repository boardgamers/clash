<script lang="ts">
  import { printedCardName } from './card-names';
  import { formatPoints } from './score';
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
    MapPin,
  } from 'lucide-svelte';
  import EventMarkers from './EventMarkers.svelte';
  import ResourceText from './ResourceText.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import { resourceNames, type Pile } from './types';
  import CivilizationAdvances from './CivilizationAdvances.svelte';
  import CityBuildings from './CityBuildings.svelte';
  import CityFacts from './CityFacts.svelte';
  import LeaderDetails from './LeaderDetails.svelte';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import { civilizationAccent } from './civilization-theme';
  import { researchPresentation } from './research';
  import { tick } from 'svelte';
  import type { Controller } from './controller';
  let { controller, onLocate }: { controller: Controller; onLocate: (position: string) => void } = $props();
  const session = $derived(controller.session);
  let player = $derived($session.view?.players.find((p) => p.index === $session.scorePlayer));
  const publicPlayer = $derived($session.game?.players.find((p) => p.id === player?.index));
  const resources = $derived(
    Object.fromEntries(
      Object.keys(resourceNames)
        .filter(
          (key) =>
            key !== 'captives' || player?.civilization === 'Aztecs' || publicPlayer?.resources?.captives,
        )
        .map((key) => [key, publicPlayer?.resources?.[key as keyof Pile] ?? 0]),
    ) as Pile,
  );
  const tab = $derived($session.scoreTab ?? 'score');
  const setTab = (scoreTab: NonNullable<typeof $session.scoreTab>) =>
    controller.patch({ scoreTab, scoreObjective: null });
  let objectiveList = $state<HTMLDivElement>();
  $effect(() => {
    const target = $session.scoreObjective;
    if (tab === 'objectives' && target) {
      void tick().then(() => {
        const card = [...(objectiveList?.querySelectorAll<HTMLElement>('[data-objective]') ?? [])].find(
          (node) => node.dataset.objective === target,
        );
        card?.scrollIntoView({ block: 'nearest' });
        card?.focus({ preventScroll: true });
      });
    }
  });
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
    style:--civilization-accent={civilizationAccent(player.civilization)}
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
    <header class="civilization-hero">
      <span class="civilization-seal"
        ><CivilizationEmblem civilization={player.civilization} size={48} /></span
      >
      <div>
        <span class="card-eyebrow">Civilization</span>
        <h2 id="score-title">{player.civilization} <EventMarkers remaining={player.eventTokens} /></h2>
      </div>
    </header>
    <div class="civilization-resources" role="group" aria-label={`${player.civilization} resources`}>
      <ResourceAmount pile={resources} showZero />
    </div>
    <nav class="civilization-tabs" aria-label="Civilization details">
      <button class:active={tab === 'score'} aria-pressed={tab === 'score'} onclick={() => setTab('score')}
        ><Trophy size={16} /> Score</button
      >
      <button
        class:active={tab === 'advances'}
        aria-pressed={tab === 'advances'}
        onclick={() => setTab('advances')}
        ><GraduationCap size={16} /> Advances <span>{advances.length}</span></button
      >
      <button
        class:active={tab === 'cities'}
        aria-pressed={tab === 'cities'}
        onclick={() => setTab('cities')}
      >
        <Landmark size={16} /> Cities <span>{player.cities.length}</span>
      </button>
      <button
        class:active={tab === 'leaders'}
        aria-pressed={tab === 'leaders'}
        onclick={() => setTab('leaders')}
      >
        <Crown size={16} /> Leaders <span>{player.civilizationLeaders?.length ?? 0}</span>
      </button>
      <button
        class:active={tab === 'objectives'}
        aria-pressed={tab === 'objectives'}
        onclick={() => setTab('objectives')}
      >
        <Target size={16} /> Objectives <span>{player.completedObjectives?.length ?? 0}</span>
      </button>
    </nav>
    {#if tab === 'score'}
      <div class="score-summary">
        <span><Trophy size={22} /> Total</span><strong>{formatPoints(player.score)}<small> VP</small></strong>
      </div>
      <div class="score-breakdown">
        {#each player.scoreParts as part}
          {@const category = categories[part.name]}
          {@const Icon = category?.icon ?? Trophy}
          <div class="score-row" class:zero={part.points === 0}>
            <Icon size={19} />
            <div>
              {#if part.name === 'Advances' || part.name === 'Objectives' || part.name === 'City pieces'}
                <button
                  class="score-category-link"
                  onclick={() =>
                    setTab(
                      part.name === 'Advances'
                        ? 'advances'
                        : part.name === 'Objectives'
                          ? 'objectives'
                          : 'cities',
                    )}
                >
                  <strong>{part.name}</strong>
                </button>
              {:else}<strong>{part.name}</strong>{/if}
              <p>{category?.description}</p>
            </div>
            <b>{formatPoints(part.points)}</b>
          </div>
        {/each}
      </div>
    {:else if tab === 'objectives'}
      <div class="public-objectives" bind:this={objectiveList}>
        {#each player.completedObjectives ?? [] as objective, index (`${objective.name}-${index}`)}
          <section
            data-objective={objective.name}
            tabindex="-1"
            class:highlighted={$session.scoreObjective === objective.name}
            aria-label={printedCardName(objective.name, 'objective')}
          >
            <header>
              <Target size={19} /><strong>{printedCardName(objective.name, 'objective')}</strong><span
                >{formatPoints(objective.points)} VP</span
              >
            </header>
            <p><ResourceText text={objective.description} /></p>
            <small>Completed</small>
          </section>
        {:else}<p class="muted">No completed objectives yet.</p>{/each}
      </div>
    {:else if tab === 'cities'}
      <div class="public-cities">
        {#each player.cities as city}
          {@const details = publicPlayer?.cities?.find((c) => c.position === city.position)}
          <section aria-label={`City ${city.position}`}>
            <header>
              <button
                title={`Show city ${city.position} on the map`}
                disabled={$session.pending}
                onclick={() => {
                  controller.patch({ scorePlayer: null, mode: 'overview' });
                  controller.selectTile(city.position, { kind: 'city' });
                }}><MapPin size={15} />{city.position}</button
              >
              <CityFacts size={city.size} mood={city.mood} />
            </header>
            {#if details}<CityBuildings
                city={details}
                owner={player.index}
                players={$session.game?.players ?? []}
                wonders={$session.view?.builtWonders}
              />{/if}
          </section>
        {:else}<p>No cities.</p>{/each}
      </div>
    {:else if tab === 'leaders'}
      <div class="public-leaders">
        {#each player.civilizationLeaders ?? [] as leader (leader.id)}
          {@const active = player?.leaders?.find((l) => l.id === leader.id)}
          <section aria-label={leader.name} class:in-play={!!active}>
            {#if leader.reason || active || leader.recruited}<div class="public-leader-status">
                <span>{leader.reason ?? (active ? 'In play' : 'Previously recruited')}</span>
                {#if active}<button
                    class="secondary"
                    disabled={$session.pending}
                    aria-label={`Show ${leader.name} on the map`}
                    onclick={() => {
                      const position = active.position;
                      close();
                      onLocate(position);
                    }}><MapPin size={15} />Show on map</button
                  >{/if}
              </div>{/if}
            <LeaderDetails {leader} />
          </section>
        {:else}<p class="muted">No leaders for this civilization.</p>{/each}
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
                  <strong>{advance.name}</strong>{#if advance.borrowed}<small
                      >{advance.borrowedSource ?? 'Great Library'}</small
                    >{/if}
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
