<script lang="ts">
  import { Landmark, Trophy, X } from 'lucide-svelte';
  import { wonderIcons } from './wonder-icons';
  import { wonderName } from './wonder-names';
  import { buildingInfo } from './city';
  import ResourceText from './ResourceText.svelte';
  import type { City, Player, View } from './types';
  let {
    city,
    owner,
    players,
    wonders = [],
  }: { city: City; owner: number; players: Player[]; wonders?: View['builtWonders'] } = $props();
  let selected = $state<{ position: string; name: string } | null>(null);
  const expanded = $derived(selected?.position === city.position ? selected.name : null);
  const wonder = $derived(wonders.find((card) => card.id === expanded));
  const effect = $derived(
    wonder?.description ??
      (expanded === 'Settlement'
        ? "The city's first piece. It adds 1 to city size, which determines collection and recruitment capacity."
        : buildingInfo[expanded ?? '']?.effect),
  );
  function toggle(name: string) {
    selected = expanded === name ? null : { position: city.position, name };
  }
  const buildings = $derived(
    Object.entries(city.city_pieces ?? {}).flatMap(([key, player]) => {
      if (typeof player !== 'number') return [];
      const name = key[0].toUpperCase() + key.slice(1);
      return [
        {
          name,
          player,
          info: buildingInfo[name],
          civilization: players.find((p) => p.id === player)?.civilization,
        },
      ];
    }),
  );
</script>

<ul class="city-buildings" aria-label={`Buildings at ${city.position}`}>
  <li>
    <button
      class:expanded={expanded === 'Settlement'}
      aria-expanded={expanded === 'Settlement'}
      onclick={() => toggle('Settlement')}><Landmark size={16} /><span>Settlement</span></button
    >
  </li>
  {#each buildings as building}
    {@const Icon = building.info?.icon ?? Landmark}
    <li>
      <button
        title={building.info?.effect}
        class:foreign-building={building.player !== owner}
        class:expanded={expanded === building.name}
        aria-expanded={expanded === building.name}
        onclick={() => toggle(building.name)}
      >
        <Icon size={16} /><span>{building.name}</span>
        {#if building.player !== owner}<small
            >· {building.civilization ?? `Player ${building.player + 1}`}</small
          >{/if}
      </button>
    </li>
  {/each}
  {#each city.city_pieces?.wonders ?? [] as id}
    {@const card = wonders.find((w) => w.id === id)}
    {@const Icon = wonderIcons[id] ?? Landmark}
    <li>
      <button class:expanded={expanded === id} aria-expanded={expanded === id} onclick={() => toggle(id)}
        ><Icon size={16} /><span>{card?.name ?? wonderName(id)}</span></button
      >
    </li>
  {/each}
</ul>

{#if expanded && effect}<section class="building-effect" aria-label={`${wonder?.name ?? expanded} effect`}>
    <header>
      <strong>{wonder?.name ?? expanded}</strong><button
        class="icon-button"
        aria-label="Close building effect"
        onclick={() => (selected = null)}><X size={15} /></button
      >
    </header>
    <p><ResourceText text={effect} /></p>
    {#if wonder}<small class="built-wonder-points"
        ><Trophy size={14} />{Math.round(wonder.builtPoints * 10) / 10} VP for building · {wonder.ownedPoints} VP
        for owning</small
      >{/if}
  </section>{/if}
