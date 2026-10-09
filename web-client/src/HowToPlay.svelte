<script lang="ts">
  import {
    X,
    Hourglass,
    Trophy,
    Landmark,
    Target,
    Hammer,
    Smile,
    Wheat,
    Coins,
    GraduationCap,
    ScrollText,
    Swords,
    Layers,
    Crown,
    Footprints,
    Mountain,
    Ship,
    Flag,
    Drama,
  } from 'lucide-svelte';
  import ResourceText from './ResourceText.svelte';
  import { ageCount, ageLabel } from './game-length';
  import type { Game } from './types';
  let { game, onClose }: { game: Game | null; onClose: () => void } = $props();
  const sections = $derived([
    {
      icon: Hourglass,
      title: 'Turns and ages',
      text: 'Each age has three rounds. Every player takes one turn per round, with three actions. Player cards show turn order and who is acting. Between ages, check objectives, gain a free advance, and draw new cards; you can also change government.',
    },
    {
      icon: Trophy,
      title: 'Victory and game end',
      text: `Most points wins. The game ends at Age ${ageLabel(ageCount(game))}’s objective checks, or at an earlier age’s checks if a player has no cities. The final age ends before free research and card draws. Select a civilization’s score for its breakdown.`,
    },
    {
      icon: Landmark,
      title: 'Scoring',
      text: 'Each settlement or building in your color scores 1 point; each advance, ½ point; each completed objective or captured leader, 2 points. Wonders and events award their listed points. Ties compare city pieces, advances, objectives, wonders, events, then captured leaders, in that order.',
    },
    {
      icon: Target,
      title: 'Objective cards',
      text: 'Each secret card normally offers two goals. Complete either for 2 points, then discard the card. The goal says whether to claim it during play or at the end of an age. Completed objectives appear in the journal and civilization details.',
    },
    {
      icon: Hammer,
      title: 'Founding and growing cities',
      text: 'Found a city for 1 action: replace a settler on empty land with a size-1 settlement. Buildings and wonders each add 1 size. After construction, a city’s size cannot exceed your total number of cities. Constructing activates the city; angry cities cannot build.',
    },
    {
      icon: Smile,
      title: 'City mood and activation',
      text: 'Collection and recruitment capacity is size + 1 when Happy, size when Neutral, and 1 when Angry. Collecting, recruiting, and constructing activate the city. Each activation after the first in a turn lowers mood one step. A city may be activated only once while Angry each turn.',
    },
    {
      icon: Smile,
      title: 'Increasing happiness',
      text: 'Spend 1 action to improve one or more cities. Each step—Angry to Neutral, or Neutral to Happy—costs mood tokens equal to the city’s size. Select cities on the map and choose their target mood. This does not activate them.',
    },
    {
      icon: Wheat,
      title: 'Collecting resources',
      text: 'Spend 1 action to activate a city and collect up to its capacity from its own or adjacent tiles. Each tile normally gives 1 food from fertile land, 1 wood from forest, or 1 ore from mountains. Advances can improve collection. Storage overflow is lost.',
    },
    {
      icon: Coins,
      title: 'Gold and storage',
      text: 'Gold replaces food, wood, ore, or ideas in payments, one for one. For example, pay 1 wood and 1 gold instead of 2 wood. The resource bar shows each storage limit. Mood tokens and culture tokens have no storage limit.',
    },
    {
      icon: GraduationCap,
      title: 'Research and events',
      text: 'Spend 1 action and 2 resources in any mix of food, ideas, and gold to research an advance. Normally, each advance uses an event marker; the last triggers an event and refills all three. Civilization advances unlock free when their prerequisites are met.',
    },
    {
      icon: Layers,
      title: 'Action cards',
      text: 'Use a card’s action effect on your turn when its conditions are met, or its battle effect with Tactics. Choose one use, then discard it. In battle, you may play one card at the start of each combat round; its effect lasts that round.',
    },
    {
      icon: Crown,
      title: 'Leaders',
      text: 'Recruit a leader for 1 mood token and 1 culture token. Recruitment costs 1 action and activates the city. You may have one leader in play. Each has two abilities and counts as an army unit, requiring Tactics to move. See all leaders in civilization details.',
    },
    {
      icon: Footprints,
      title: 'Moving groups',
      text: 'Spend 1 action to move up to three groups of units. Each group moves together from one tile; each unit may join one group per Move action. Land groups move to an adjacent tile unless an ability extends their range. Army movement requires Tactics.',
    },
    {
      icon: Mountain,
      title: 'Terrain and exploration',
      text: 'Entering unexplored land reveals it. Entering Mountains or fighting a battle stops those units from moving again that turn. After entering Forest, units can move again but cannot attack that turn. Roads can bypass Forest and Mountain restrictions. Another Move action allows otherwise eligible units to move again.',
    },
    {
      icon: Ship,
      title: 'Ships and sea movement',
      text: 'Recruit ships at a Port for 2 wood each; they start on its sea tile. A fleet can cross connected sea tiles as one group. Exploration ends its move on a connected sea tile in the new region. Navigation allows travel around the map edge.',
    },
    {
      icon: Ship,
      title: 'Transporting units',
      text: 'Each ship carries two land units. Boarding from adjacent land or landing onto it uses that land group’s move. Sailing carries passengers along. Boarding and sailing can share one Move action; landing those units needs another. Units already aboard can sail and land in one action.',
    },
    {
      icon: Swords,
      title: 'Battles',
      text: 'Enter an enemy tile with an army or fleet to fight; settlers do not fight. Each round, roll one die per fighting unit and apply bonuses. Every 5 combat value deals a hit before cancellations. Both sides choose casualties. The attacker may then retreat if allowed; otherwise continue until one side is defeated.',
    },
    {
      icon: Flag,
      title: 'Capturing cities',
      text: 'An army takes an undefended city or one whose defenders it defeats. It becomes Angry; change its pieces to your color, except obelisks and third-party buildings. From a player’s city, gain gold equal to its size (+1 if Happy; only 1 gold if Angry). Its former owner places a settler in a remaining city, if able.',
    },
    {
      icon: Drama,
      title: 'Cultural influence',
      text: 'Spend 1 action to target a building within range equal to your city’s size. A roll of 5+ changes it to your color and scores its point; the city’s owner still uses it. Each culture token adds 1 range before rolling or +1 afterwards. Only one successful influence per turn.',
    },
    {
      icon: ScrollText,
      title: 'Barbarians',
      text: 'Barbarians block collection on their tile. Events can spawn, move, or reinforce them; cavalry or elephants may be placed in a city that already has infantry. Winning a battle against barbarians gives 1 gold total, whether attacking or defending. Capturing a barbarian city gives 1 gold in addition to any battle reward.',
    },
    {
      icon: Ship,
      title: 'Pirates',
      text: 'Pirates block sea collection and Trade Routes on their tile and adjacent sea tiles. Attack by moving or recruiting ships onto their tile. For each pirate ship destroyed in battle, gain 1 gold and your choice of 1 mood token or 1 culture token, even if you lose. Pirates controlled by Carthage give no reward.',
    },
  ]);
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide how-to-play"
  aria-labelledby="how-to-play-title"
  use:show
  onclose={onClose}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onClose();
  }}
>
  <button class="close-guide icon-button" aria-label="Close field guide" onclick={onClose}
    ><X size={20} /></button
  >
  <h2 id="how-to-play-title">How to play</h2>
  <p class="guide-intro">Build your civilization over {ageCount(game)} ages. Most victory points wins.</p>
  {#if game?.options?.variant === 'Builder'}
    <p class="guide-intro">
      <strong>Builder:</strong> No military attacks against other players. Cultural Influence is allowed; barbarians
      and pirates remain. Objectives that require player battles are removed.
    </p>
  {/if}
  <div class="guide-grid">
    {#each sections as section}
      {@const Icon = section.icon}
      <article>
        <Icon />
        <h3>{section.title}</h3>
        <p><ResourceText text={section.text} namedResources /></p>
      </article>
    {/each}
  </div>
</dialog>
