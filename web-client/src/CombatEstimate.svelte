<script lang="ts">
  import EstimateWorker from './combat-estimate.worker?worker&inline';
  import wasm from '../.bridge/server_bg.wasm?url&no-inline';
  import type { Controller } from './controller';
  import type { Move } from './types';
  import {
    cachedEstimate,
    cacheEstimate,
    percentages,
    type CombatEstimate,
    type CombatScenario,
  } from './combat-estimate';
  import ResourceAmount from './ResourceAmount.svelte';
  let {
    controller,
    action,
    siege = false,
    steel = false,
  }: { controller: Controller; action: Move; siege?: boolean; steel?: boolean } = $props();
  const session = $derived(controller.session);
  let results = $state<(CombatEstimate | undefined)[]>([]);
  let errors = $state<(string | undefined)[]>([]);
  let done = $state(false);
  let expanded = $state(false);
  const scenarios = $derived<CombatScenario[]>([
    { label: siege ? 'No Siegecraft' : 'Battle estimate', siege: 0, steel: true },
    ...(steel ? [{ label: 'Without your Steel Weapons', siege: 0, steel: false }] : []),
    ...(siege
      ? [
          { label: 'Cancel extra die', siege: 1, steel: true },
          { label: 'Cancel ignored hit', siege: 2, steel: true },
          { label: 'Cancel both', siege: 3, steel: true },
        ]
      : []),
  ]);
  $effect(() => {
    // Subscribe to game replacements as well as selection changes; never show stale odds.
    const game = $session.game;
    const currentAction = action;
    const variants = scenarios;
    results = [];
    errors = [];
    done = false;
    if (!game || $session.playback || $session.pending) return;
    const snapshot = controller.combatEstimateSnapshot();
    if (!snapshot) return;
    const key = JSON.stringify([snapshot.raw, snapshot.seat, currentAction, variants]);
    const cached = cachedEstimate(key);
    if (cached) {
      results = cached.results;
      errors = cached.errors;
      done = true;
      return;
    }
    let worker: Worker | undefined;
    const timer = setTimeout(() => {
      try { worker = new EstimateWorker(); }
      catch { errors = ['Background simulation is unavailable in this browser.']; done = true; return; }
      worker.onmessage = ({ data }) => {
        results = data.results ?? [];
        errors = data.errors ?? [data.error];
        done = data.done;
        if (data.done) {
          cacheEstimate(key, { results: data.results ?? [], errors: data.errors ?? [data.error] });
          worker?.terminate();
        }
      };
      worker.onerror = () => {
        results = [];
        errors = ['Estimate unavailable'];
        done = true;
        worker?.terminate();
      };
      worker.postMessage({
        ...snapshot,
        action: currentAction,
        scenarios: variants,
        wasm: new URL(wasm, location.href).href,
      });
    }, 200);
    return () => {
      clearTimeout(timer);
      worker?.terminate();
    };
  });
</script>

<div class="combat-estimate" aria-label="Estimated combat outcome">
  <div class="estimate-overview">
  {#if results[0] && results[0].samples >= 256}
    {@const odds = percentages(results[0])}
    <div class="estimate-summary">
      <strong>Win ≈{odds[0]}%</strong><span>Draw {odds[1]}%</span><span>Loss {odds[2]}%</span>
    </div>
  {:else if done}<span class="estimate-losses" title={errors[0]}
      >{errors[0] === 'No battle to estimate'
        ? 'Undefended capture · no battle'
        : 'Estimate unavailable for this battle.'}</span
    >
  {:else}<span class="estimate-losses">Estimating battle…</span>{/if}
  <button class="estimate-details" onclick={() => (expanded = !expanded)} aria-expanded={expanded} title="Assumptions & options"
    >{expanded ? 'Hide' : 'Details'}{!done && (results[0]?.samples ?? 0) >= 256
      ? ' · refining…'
      : ''}</button
  >
  </div>
  {#if expanded}
    {#if results[0] && results[0].samples >= 256}
    <div class="estimate-losses">
      Average losses: you {(results[0].ownLosses / results[0].samples).toFixed(1)} · enemy {(
        results[0].enemyLosses / results[0].samples
      ).toFixed(1)}
    </div>
    {/if}
    {#if errors[0]}<p>{errors[0]}</p>{/if}
    <p>
      Selected option included. No retreat or additional tactics; hidden tactics ignored. Steel Weapons when
      affordable. Other optional abilities declined. Infantry lost before cavalry, elephants and leaders.
    </p>
    {#if scenarios.length > 1}
      <div class="estimate-options">
        {#each scenarios as scenario, i}
          <div class="estimate-option">
            <span>{scenario.label}</span>
            {#if results[i] && results[i]!.samples >= 256}<strong>≈{percentages(results[i]!)[0]}% win</strong
              >{#if Object.values(results[i]!.cost).some(Boolean)}<ResourceAmount
                  pile={results[i]!.cost}
                />{:else}<span>Free</span>{/if}
            {:else if errors[i]}<small title={errors[i]}>Unavailable</small>{:else}<small>Calculating…</small
              >{/if}
          </div>
        {/each}
      </div>
      <p>Costs shown are Steel Weapons and Siegecraft payments. Comparisons do not select or spend them.</p>
    {/if}
    <p>
      {results[0]?.samples ?? 0} simulations{done ? '' : ' · refining'} · Estimates depend on choices; losses from
      this choice include passengers.
    </p>
  {/if}
</div>

<style>
  .combat-estimate {
    border-top: 1px solid var(--line);
    padding-top: 0.6rem;
    margin-top: 0.6rem;
    font-size: 12px;
  }
  .estimate-overview {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.25rem 0.75rem;
    flex-wrap: wrap;
  }
  .estimate-summary {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.45rem 1rem;
  }
  .estimate-summary strong {
    color: var(--text);
    font-size: 14px;
  }
  .estimate-losses,
  p {
    color: var(--muted);
    font-size: 11px;
    margin: 0.35rem 0;
  }
  .estimate-details {
    background: none;
    border: 0;
    padding: 0.2rem 0;
    color: var(--muted);
    font-size: 11px;
    text-decoration: underline;
  }
  .estimate-options {
    display: grid;
    gap: 0.4rem;
    margin: 0.5rem 0;
  }
  .estimate-option {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.35rem 0.65rem;
  }
  .estimate-option > span:first-child {
    flex: 1;
    min-width: 140px;
  }
  .estimate-option small {
    color: var(--muted);
  }
</style>
