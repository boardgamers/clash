<script lang="ts">
  import { X, Landmark, Hammer, Users, Smile, Plus, Minus, Check } from 'lucide-svelte';
  import type { Controller } from './controller';
  import { buildingInfo, unitInfo, cityReason } from './city';
  import ResourceAmount from './ResourceAmount.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let tab = $state('build');
  let building = $state<string | null>(null);
  let city = $derived($session.view?.cities.find((c) => c.position === $session.city));
  let options = $derived($session.view?.cityActions.find((c) => c.position === $session.city));
  let selected = $derived(options?.buildings.find((b) => b.name === building));
  let count = $derived(Object.values($session.recruits).reduce((a, b) => a + (b ?? 0), 0));
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
      <p>Size {city?.size} · {city?.mood} · Collect or recruit up to {city?.capacity}</p>
    </div>
    <button class="icon-button" aria-label="Close city management" onclick={close}><X /></button>
  </header>
  <nav class="city-picker" aria-label="Choose city">
    {#each $session.view?.cities ?? [] as c}<button
        class:active={c.position === $session.city}
        onclick={() => {
          building = null;
          controller.openCities(c.position);
        }}><Landmark size={14} />{c.position} <span>{c.mood}</span></button
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
    {#if city}<ActivationStatus
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
                  amount >= item.available ||
                  count >= (city?.capacity ?? 0) ||
                  $session.pending}
                onclick={() => controller.setRecruits({ ...$session.recruits, [info.key]: amount + 1 })}
                ><Plus size={15} /></button
              >
            </div>
          </article>{/each}
      </div>
    {:else}
      <p class="city-rule">
        Happier cities collect more and recruit more. Improving mood uses 1 action without activating the
        city.
      </p>
      {#if !options?.happiness.length}<div class="happy-message">
          <Smile size={32} />
          <h3>This city is already happy</h3>
          <p>Its capacity is {city?.capacity}, one more than its size.</p>
        </div>{/if}
      <div class="happiness-options">
        {#each options?.happiness ?? [] as item}<article>
            <Smile size={26} />
            <h3>{item.mood}</h3>
            <p>Raise mood by {item.steps} {item.steps === 1 ? 'step' : 'steps'}.</p>
            <ResourceAmount pile={item.payment} /><small>{cityReason(item.reason) || 'Costs 1 action'}</small
            ><button
              class="primary"
              title={item.reason ?? undefined}
              disabled={!item.action || $session.pending}
              onclick={() => item.action && controller.submit(item.action)}>Raise to {item.mood}</button
            >
          </article>{/each}
      </div>
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
