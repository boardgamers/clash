<script lang="ts">
  import { Compass, RotateCw, Check, LocateFixed } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { positionXY } from './model';
  import TerrainIcon from './TerrainIcon.svelte';
  import { terrainInfo } from './terrain';
  let { controller, onLocate }: { controller: Controller; onLocate: () => void } = $props();
  const session = $derived(controller.session);
  let decision = $derived($session.view?.explorationDecision);
  let selected = $derived(
    decision?.choices.find((c) => c.rotation === $session.explorationRotation) ?? decision?.choices[0],
  );
  const hex = Array.from(
    { length: 6 },
    (_, i) => `${Math.cos((i * Math.PI) / 3) * 0.96},${Math.sin((i * Math.PI) / 3) * 0.96}`,
  ).join(' ');
</script>

{#if decision}
  <section class="action-panel floating-panel exploration-panel" aria-label="Explore terrain placement">
    <header class="exploration-heading">
      <h2><Compass size={20} />Explore</h2>
      <button class="icon-button" aria-label="Locate the region being explored" onclick={onLocate}
        ><LocateFixed size={18} /></button
      >
    </header>
    <div class="exploration-choices">
      {#each decision.choices as choice, i}
        {@const destinationTerrain = choice.tiles.find(
          ([position]) => position === decision.destination,
        )?.[1]}
        {@const points = choice.tiles.map(([position]) => positionXY(position))}
        {@const minX = Math.min(...points.map((p) => p[0])) - 1.1}
        {@const minY = Math.min(...points.map((p) => p[1])) - 1.1}
        {@const width = Math.max(...points.map((p) => p[0])) - minX + 1.1}
        {@const height = Math.max(...points.map((p) => p[1])) - minY + 1.1}
        <button
          class:selected={selected?.rotation === choice.rotation}
          aria-pressed={selected?.rotation === choice.rotation}
          aria-label={`Terrain placement ${i + 1}`}
          disabled={$session.pending}
          onmouseenter={() => controller.patch({ explorationPreview: choice.rotation })}
          onmouseleave={() => controller.patch({ explorationPreview: null })}
          onfocus={() => controller.patch({ explorationPreview: choice.rotation })}
          onblur={() => controller.patch({ explorationPreview: null })}
          onclick={() => controller.patch({ explorationRotation: choice.rotation, explorationPreview: null })}
        >
          <span class="placement-heading"
            ><RotateCw size={14} />{i + 1}{#if selected?.rotation === choice.rotation}<Check
                size={14}
              />{/if}</span
          >
          <svg class="placement-map" viewBox={`${minX} ${minY} ${width} ${height}`} aria-hidden="true">
            {#each choice.tiles as [position, terrain], t}{@const info = terrainInfo(terrain)}
              <g transform={`translate(${points[t][0]} ${points[t][1]})`}>
                <polygon
                  points={hex}
                  fill={info.color}
                  stroke={position === decision.destination ? '#263f33' : '#ffffff'}
                  stroke-width={position === decision.destination ? 0.1 : 0.03}
                />
                <g transform="translate(-.27,-.27)"
                  ><info.Icon size={0.54} strokeWidth={1.7} color="#263f33" /></g
                >
              </g>
            {/each}
          </svg>
          <span class="sr-only"
            >{choice.tiles.map(([, terrain]) => terrainInfo(terrain).label).join(', ')}</span
          >
          {#if decision.destination && destinationTerrain}<span
              class="placement-destination"
              title={`Arrival terrain: ${terrainInfo(destinationTerrain).label}`}
              ><TerrainIcon terrain={destinationTerrain} size={16} />Arrive here</span
            >{/if}
        </button>
      {/each}
    </div>
    <div class="exploration-confirm">
      <span>Orient the highlighted region</span><button
        class="primary"
        disabled={!selected || $session.pending}
        onclick={() => selected && controller.submit(selected.action)}>Confirm <Check size={16} /></button
      >
    </div>
    {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
  </section>
{/if}
