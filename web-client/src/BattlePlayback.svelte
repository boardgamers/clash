<script lang="ts">
  import { Swords, Shield, X } from 'lucide-svelte';
  import type { Controller } from './controller';
  import CivilizationEmblem from './CivilizationEmblem.svelte';
  import UnitIcon from './UnitIcon.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  const cue = $derived($session.battles?.[0]);
  const motion = $derived($session.battleAnimate && !$session.reducedMotion);
</script>

{#if cue}{#key cue.key}
    <section
      class="battle-recap"
      class:battle-motion={motion}
      class:battle-roll={motion && cue.roll}
      aria-label="Battle summary"
    >
      <header>
        <strong
          ><Swords size={16} />Battle{#if cue.combat.round}
            · Round {cue.combat.round}{/if}</strong
        >
        {#if ($session.battles?.length ?? 0) > 1}<small>{($session.battles?.length ?? 1) - 1} more</small
          >{/if}
        <button
          class="icon-button"
          aria-label="Dismiss battle summary"
          onclick={() => controller.dismissBattles()}><X size={15} /></button
        >
      </header>
      <div class="battle-sides">
        {#each [cue.combat.attacker, cue.combat.defender] as side, index}
          <div class="battle-side">
            <div class="battle-side-name">
              {#if index === 0}<Swords size={12} />{:else}<Shield size={12} />{/if}
              {#if side.civilization}<CivilizationEmblem
                  civilization={side.civilization}
                  size={17}
                />{/if}<strong>{side.civilization ?? (index ? 'Defender' : 'Attacker')}</strong>
            </div>
            {#if side.dice}
              <div class="battle-dice" aria-label={`${side.civilization ?? ''} recorded dice`}>
                {#each side.dice as die, i}<span
                    class="battle-die"
                    class:activated={!!die.effect}
                    style={`--die-delay:${i * 45}ms`}
                    title={`${die.value}${die.symbol ? ` · ${die.symbol}` : ''}${die.effect ? ` · ${die.effect}` : ''}`}
                  >
                    <b>{die.value}</b>{#if die.symbol}<UnitIcon
                        type={die.symbol === 'Leader' ? { Leader: 'Leader' } : die.symbol}
                        size={12}
                      />{/if}
                  </span>{:else}<small>No dice</small>{/each}
              </div>
              <div class="battle-hit-count">
                <span>{side.value} value</span><strong
                  >{#if side.cancelledHits}<s title={side.cancelledHits.reasons.join(' · ')}
                      >{side.cancelledHits.before}</s
                    >{/if}
                  {side.hits}
                  {side.hits === 1 ? 'hit' : 'hits'}</strong
                >
              </div>
            {:else}<small class="battle-preparing"
                >Preparing battle{#if side.units?.length}
                  · {side.units.reduce((sum, u) => sum + u.count, 0)} units{/if}</small
              >{/if}
          </div>
        {/each}
      </div>
      {#if cue.combat.result || cue.losses.length}<footer class="battle-outcome" aria-live="polite">
          {#if cue.combat.result}<strong>{cue.combat.result}</strong>{/if}
          {#each cue.losses as loss}<span>{loss.civilization} {loss.value} {loss.label}</span>{/each}
        </footer>{/if}
    </section>
  {/key}{/if}
