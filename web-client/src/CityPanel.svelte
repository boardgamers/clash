<script lang="ts">
  import { X, Landmark, Hammer, Users, Smile, Plus, Minus, Check, Crown } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { buildingInfo, unitInfo, cityReason } from './city';
  import ResourceAmount from './ResourceAmount.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  import HappinessPanel from './HappinessPanel.svelte';
  import CityFacts from './CityFacts.svelte';
  import UnitIcon from './UnitIcon.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let tab = $state('build');
  let building = $state<string | null>(null);
  let city = $derived($session.view?.cities.find((c) => c.position === $session.city));
  let options = $derived($session.view?.cityActions.find((c) => c.position === $session.city));
  let selected = $derived(options?.buildings.find((b) => b.name === building));
  let count = $derived(
    Object.values($session.recruits).reduce<number>((a, b) => a + (typeof b === 'string' ? 1 : (b ?? 0)), 0),
  );
  function close() {
    controller.patch({ mode: 'overview', error: '' });
  }
  function open(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
</script>

<dialog
  class="city-dialog"
  use:open
  onclose={close}
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
  aria-labelledby="city-title"
>
  <header class="city-dialog-header">
    <div>
      <h2 id="city-title"><Landmark size={26} /> City {$session.city}</h2>
      <p>
        {#if city}<CityFacts size={city.size} mood={city.mood} />{/if} · Capacity {city?.capacity}
      </p>
    </div>
    <button class="icon-button" aria-label="Close city management" onclick={close}><X /></button>
  </header>
  <nav class="city-picker" aria-label="Choose city">
    {#each $session.view?.cities ?? [] as c}<button
        class:active={c.position === $session.city}
        onclick={() => {
          building = null;
          controller.openCities(c.position);
        }}><Landmark size={14} />{c.position}<CityFacts size={c.size} mood={c.mood} /></button
      >{/each}
  </nav>
  <nav class="city-tabs" aria-label="City actions">
    {#each [{ id: 'build', label: 'Buildings', icon: Hammer }, { id: 'recruit', label: 'Recruit', icon: Users }, { id: 'happiness', label: 'Happiness', icon: Smile }] as item}<button
        class:active={tab === item.id}
        onclick={() => {
          tab = item.id;
          controller.patch({ error: '' });
        }}><item.icon size={17} />{item.label}</button
      >{/each}
  </nav>
  <div class="city-content">
    {#if city && tab !== 'happiness'}<ActivationStatus
        {city}
        warning={tab === 'recruit' || (tab === 'build' && !!selected?.moodWillDecrease)}
      />{/if}
    {#if tab === 'build'}
      <p class="city-rule">
        Each building adds 1 city size and 1 point. A city’s size cannot exceed your number of cities.
      </p>
      <div class="building-grid">
        {#each options?.buildings ?? [] as item}{@const info = buildingInfo[item.name]}<button
            class="building-option"
            class:owned={item.owned}
            class:selected={building === item.name}
            onclick={() => (building = item.name)}
            aria-pressed={building === item.name}
            ><strong
              ><info.icon size={21} />{item.name}{#if item.owned}<Check size={16} />{/if}</strong
            ><span>{info.effect}</span>{#if item.owned}<small>Built</small>{:else}<ResourceAmount
                pile={item.payment}
              />{#if cityReason(item.reason, city?.size) || item.choices.length}<small
                  >{cityReason(item.reason, city?.size) || 'Available'}</small
                >{/if}{/if}</button
          >{/each}
      </div>
    {:else if tab === 'recruit'}
      <p class="city-rule">
        Recruit up to {city?.capacity} units together for 1 action. Activates this city.
      </p>
      <div class="recruit-list">
        {#each options?.recruits ?? [] as item}{@const info = unitInfo[item.type]}{@const amount =
            $session.recruits[info.key] ?? 0}
          <article class="recruit-row">
            <info.icon size={24} />
            <div>
              <strong>{item.type}</strong>
              <p>{info.effect}</p>
              <ResourceAmount pile={item.payment} /><small
                >{cityReason(item.reason) || `${item.available} in supply`}</small
              >
            </div>
            <div class="quantity">
              <button
                aria-label={`Remove ${item.type}`}
                disabled={!amount || $session.pending}
                onclick={() => controller.setRecruits({ ...$session.recruits, [info.key]: amount - 1 })}
                ><Minus size={15} /></button
              ><output aria-label={`${item.type} selected`}>{amount}</output><button
                aria-label={`Add ${item.type}`}
                disabled={!!item.reason ||
                  amount >= (item.limit ?? item.available) ||
                  count >= (city?.capacity ?? 0) ||
                  $session.pending}
                onclick={() => controller.setRecruits({ ...$session.recruits, [info.key]: amount + 1 })}
                ><Plus size={15} /></button
              >
            </div>
          </article>{/each}
      </div>
      {#if options?.leaders?.length}<section class="leader-recruit" aria-label="Leaders">
          <h3><Crown size={16} />Leaders</h3><ResourceAmount
            pile={{ mood_tokens: 1, culture_tokens: 1 }}
          />
          <div class="unit-picker">
            {#each options.leaders as leader}<button
                title={leader.description}
                class:selected={$session.recruits.leader === leader.id}
                aria-pressed={$session.recruits.leader === leader.id}
                disabled={$session.pending || (!$session.recruits.leader && count >= (city?.capacity ?? 0))}
                onclick={() =>
                  controller.setRecruits({
                    ...$session.recruits,
                    leader: $session.recruits.leader === leader.id ? null : leader.id,
                  })}><Crown size={15} />{leader.name}</button
              >{/each}
          </div>
        </section>{/if}
      {#if $session.view?.units?.length}<details class="replacement-recruit">
          <summary>Replace units on the map</summary>
          <div class="unit-picker">
            {#each $session.view.units as unit}<button
                class:selected={$session.replacements.includes(unit.id)}
                aria-pressed={$session.replacements.includes(unit.id)}
                title={`Replace ${typeof unit.type === 'string' ? unit.type : unit.type.Leader} #${unit.id + 1} at ${unit.position}`}
                onclick={() => {
                  controller.patch({
                    replacements: $session.replacements.includes(unit.id)
                      ? $session.replacements.filter((id) => id !== unit.id)
                      : [...$session.replacements, unit.id],
                  });
                  controller.setRecruits($session.recruits);
                }}><UnitIcon type={unit.type} />#{unit.id + 1} · {unit.position}</button
              >{/each}
          </div>
        </details>{/if}
    {:else}
      <HappinessPanel {controller} />
    {/if}
  </div>
  {#if tab === 'build' && selected && !selected.owned}<footer class="city-confirm">
      <div>
        <strong>{selected.name}</strong><ResourceAmount pile={selected.payment} /><small
          >{cityReason(selected.reason, city?.size) || 'Costs 1 action · Activates this city'}</small
        >{#if selected.moodWillDecrease && city && city.activationMood !== city.mood}<span
            class="activation-inline"
            ><strong>{city.mood} → {city.activationMood}</strong> after activation</span
          >{/if}
      </div>
      <div class="port-choices">
        {#each selected.choices as choice}<button
            class="primary"
            disabled={$session.pending}
            onclick={() => controller.submit(choice.action)}
            >Build {selected.name}{choice.position ? ` at ${choice.position}` : ''}</button
          >{/each}
      </div>
    </footer>{/if}
  {#if tab === 'recruit'}<footer class="city-confirm">
      <div>
        <strong>{count} {count === 1 ? 'unit' : 'units'} selected</strong
        >{#if $session.recruitPreview}<ResourceAmount
            pile={$session.recruitPreview.payment}
          />{#if $session.recruitPreview.moodWillDecrease && city && city.activationMood !== city.mood}<span
              class="activation-inline"
              ><strong>{city.mood} → {city.activationMood}</strong> after activation</span
            >{/if}{/if}
      </div>
      <button
        class="primary"
        disabled={!$session.recruitPreview || $session.pending}
        onclick={() => $session.recruitPreview && controller.submit($session.recruitPreview.action)}
        >{$session.pending
          ? 'Confirming…'
          : `Recruit ${count || ''} ${count === 1 ? 'unit' : 'units'}`}</button
      >
    </footer>{/if}
  {#if $session.error}<p class="city-error" role="alert">{cityReason($session.error, city?.size)}</p>{/if}
</dialog>
