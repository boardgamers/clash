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
  import type { Controller } from './controller';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let player = $derived($session.view?.players.find((p) => p.index === $session.scorePlayer));
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
    <button class="close-guide icon-button" aria-label="Close score breakdown" onclick={close}
      ><X size={20} /></button
    >
    <span class="card-eyebrow">Victory points</span>
    <h2 id="score-title">{player.civilization}</h2>
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
    <button class="score-profile secondary" onclick={() => controller.commands.openPlayer(player!.index)}
      ><UserRound size={15} /> {player.name} · Player profile</button
    >
  </dialog>
{/if}
