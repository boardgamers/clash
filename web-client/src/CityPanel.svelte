<script lang="ts">
  import {
    X,
    Landmark,
    Hammer,
    Users,
    Smile,
    Plus,
    Minus,
    Check,
    Crown,
    Swords,
    Sparkles,
    GraduationCap,
    Layers,
    Footprints,
  } from 'lucide-svelte';
  import type { Move, Pile } from './types';
  import { pileText } from './model';
  import type { Controller } from './controller';
  import { buildingInfo, unitInfo, cityReason } from './city';
  import ResourceAmount from './ResourceAmount.svelte';
  import ResourceText from './ResourceText.svelte';
  import ActivationStatus from './ActivationStatus.svelte';
  import HappinessPanel from './HappinessPanel.svelte';
  import CityFacts from './CityFacts.svelte';
  import UnitIcon from './UnitIcon.svelte';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let tab = $derived($session.cityTab);
  let building = $state<string | null>(null);
  let paymentIndex = $state(-1);
  function submitBuild(action: Move) {
    const chosen = structuredClone(action) as { Playing: { Construct: { payment: Pile } } };
    if (buildPayment) chosen.Playing.Construct.payment = buildPayment;
    controller.submit(chosen);
  }
  let city = $derived($session.view?.cities.find((c) => c.position === $session.city));
  let options = $derived($session.view?.cityActions.find((c) => c.position === $session.city));
  let selected = $derived(options?.buildings.find((b) => b.name === building));
  const buildActivatesCity = $derived(
    selected
      ? selected.activateCity !== false
      : (options?.buildings.some((b) => b.activateCity !== false) ?? true),
  );
  const buildSummary = $derived(
    [
      selected?.source,
      selected?.free ? 'No extra action' : 'Costs 1 action',
      selected?.activateCity === false ? 'No city activation' : 'Activates this city',
    ]
      .filter(Boolean)
      .join(' · '),
  );
  const buildPayment = $derived(
    paymentIndex >= 0 ? (selected?.payments?.[paymentIndex] ?? selected?.payment) : selected?.payment,
  );

  let count = $derived(
    Object.values($session.recruits).reduce<number>((a, b) => a + (typeof b === 'string' ? 1 : (b ?? 0)), 0),
  );
  const capacity = $derived((city?.capacity ?? 0) + Number(!!$session.ballcourts && !!city?.ballcourts));
  const occupiedCapacity = $derived(count + Number(!!$session.draftCard));
  function abilityIcon(text: string) {
    if (/battle|combat|captur|attack/i.test(text)) return Swords;
    if (/advance/i.test(text)) return GraduationCap;
    if (/wonder/i.test(text)) return Landmark;
    if (/card/i.test(text)) return Layers;
    if (/happiness/i.test(text)) return Smile;
    if (/mov(e|ing|ement)/i.test(text)) return Footprints;
    return Sparkles;
  }
  function close() {
    // A confirmed recruitment may already have opened bonus movement controls.
    if ($session.mode === 'city') controller.patch({ mode: 'overview', error: '' });
  }
  function open(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
</script>

<dialog
  class="city-dialog"
  class:happiness-dialog={tab === 'happiness'}
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
      <h2 id="city-title">
        {#if tab === 'happiness'}<Smile size={26} />Happiness{:else}<Landmark size={26} />City {$session.city}{/if}
      </h2>
      {#if tab !== 'happiness'}<p>
          {#if city}<CityFacts size={city.size} mood={city.mood} />{/if} · Capacity {city?.capacity}
        </p>{/if}
    </div>
    {#if tab === 'happiness'}<div class="happiness-budget" title="Mood tokens available">
        <small>Available</small><ResourceAmount
          pile={{
            mood_tokens:
              $session.game?.players.find((p) => p.id === $session.seat)?.resources?.mood_tokens ?? 0,
          }}
          showZero
        />
      </div>{/if}
    <button class="icon-button" aria-label="Close city management" onclick={close}><X /></button>
  </header>
  {#if tab !== 'happiness' && ($session.view?.cities.length ?? 0) > 1}<nav
      class="city-picker"
      aria-label="Choose city"
    >
      {#each $session.view?.cities ?? [] as c}<button
          class:active={c.position === $session.city}
          onclick={() => {
            building = null;
            controller.openCities(c.position);
          }}><Landmark size={14} />{c.position}<CityFacts size={c.size} mood={c.mood} /></button
        >{/each}
    </nav>{/if}
  <nav class="city-tabs" aria-label="City actions">
    {#each [{ id: 'build', label: 'Buildings', icon: Hammer }, { id: 'recruit', label: 'Recruit', icon: Users }, { id: 'happiness', label: 'Happiness', icon: Smile }] as item}<button
        class:active={tab === item.id}
        onclick={() => {
          controller.patch({ cityTab: item.id as typeof tab });
          controller.patch({ error: '' });
        }}><item.icon size={17} />{item.label}</button
      >{/each}
  </nav>
  <div class="city-content">
    {#if city && (tab === 'recruit' || (tab === 'build' && buildActivatesCity))}<ActivationStatus
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
            onclick={() => {
              building = item.name;
              paymentIndex = -1;
            }}
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
        Recruit up to {(city?.capacity ?? 0) + Number(!!$session.ballcourts && !!city?.ballcourts)} units together
        for 1 action. Activates this city.
      </p>
      {#if city?.ballcourts}<label class="ballcourts-toggle"
          ><input
            type="checkbox"
            checked={$session.ballcourts ?? false}
            onchange={(e) => controller.setBallcourts(e.currentTarget.checked)}
          />
          Ballcourts · +1 capacity <ResourceAmount pile={{ mood_tokens: 1 }} /></label
        >{/if}
      {#if city?.shogunateDraft}<label class="ballcourts-toggle"
          ><input
            type="checkbox"
            checked={$session.draftCard ?? false}
            disabled={!$session.draftCard && occupiedCapacity >= capacity}
            onchange={(e) => controller.setDraftCard(e.currentTarget.checked)}
          />
          Shogunate · Draft an action card <ResourceAmount pile={{ mood_tokens: 1 }} /></label
        >{/if}
      {#if city?.piratePort}<label class="ballcourts-toggle"
          ><input
            type="checkbox"
            checked={$session.attackPirates ?? false}
            onchange={(e) => {
              controller.patch({ attackPirates: e.currentTarget.checked });
              controller.setRecruits($session.recruits);
            }}
          /> Attack pirates with recruited Ships</label
        >{/if}
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
                  occupiedCapacity >= capacity ||
                  $session.pending}
                onclick={() => controller.setRecruits({ ...$session.recruits, [info.key]: amount + 1 })}
                ><Plus size={15} /></button
              >
            </div>
          </article>{/each}
      </div>
      {#if options?.leaders?.length}<section class="leader-recruit" aria-label="Leaders">
          <h3><Crown size={16} />Leaders</h3>
          <div class="leader-grid">
            {#each options.leaders as leader}
              <article class="leader-card" class:selected={$session.recruits.leader === leader.id}>
                <button
                  class="leader-select"
                  aria-label={`Select ${leader.name}`}
                  aria-pressed={$session.recruits.leader === leader.id}
                  disabled={$session.pending ||
                    (!!leader.reason && $session.recruits.leader !== leader.id) ||
                    (!$session.recruits.leader && occupiedCapacity >= capacity)}
                  title={leader.reason ?? undefined}
                  onclick={() =>
                    controller.setRecruits({
                      ...$session.recruits,
                      leader: $session.recruits.leader === leader.id ? null : leader.id,
                    })}
                >
                  <span class="leader-name"><Crown size={16} />{leader.name}</span>
                  <span class="leader-cost"
                    ><span>Cost</span><ResourceAmount
                      pile={leader.payment ?? { mood_tokens: 1, culture_tokens: 1 }}
                    />
                    <span class="leader-selection"
                      >{#if $session.recruits.leader === leader.id}<Check size={14} />Selected{:else}<Plus
                          size={14}
                        />Select{/if}</span
                    >
                  </span>
                </button>
                {#if leader.reason}<small class="reason">{leader.reason}</small>{/if}
                <dl class="leader-abilities">
                  {#each leader.abilities as ability}{@const Icon = abilityIcon(ability.description)}
                    <div>
                      <dt><Icon size={15} />{ability.name}</dt>
                      <dd><ResourceText text={ability.description} /></dd>
                    </div>
                  {/each}
                </dl>
              </article>
            {/each}
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
        <strong>{selected.name}</strong><ResourceAmount pile={buildPayment ?? selected.payment} /><small
          >{cityReason(selected.reason, city?.size) || buildSummary}</small
        >{#if selected.moodWillDecrease && city && city.activationMood !== city.mood}<span
            class="activation-inline"
            ><strong>{city.mood} → {city.activationMood}</strong> after activation</span
          >{/if}
      </div>
      {#if selected.payments && selected.payments.length > 1}<label class="build-payment"
          >Pay with
          <select
            aria-label={`Payment for ${selected.name}`}
            value={paymentIndex}
            onchange={(e) => (paymentIndex = Number(e.currentTarget.value))}
          >
            <option value={-1}>{pileText(selected.payment)}</option>
            {#each selected.payments as payment, i}<option value={i}>{pileText(payment)}</option>{/each}
          </select>
        </label>{/if}
      <div class="port-choices">
        {#each selected.choices as choice}<button
            class="primary"
            disabled={$session.pending}
            onclick={() => submitBuild(choice.action)}
            >Build {selected.name}{choice.position ? ` at ${choice.position}` : ''}</button
          >{/each}
      </div>
    </footer>{/if}
  {#if tab === 'recruit'}<footer class="city-confirm">
      <div>
        <strong
          >{count} {count === 1 ? 'unit' : 'units'}{$session.draftCard ? ' + 1 card' : ''} selected</strong
        >{#if $session.recruitPreview}<ResourceAmount
            pile={$session.recruitPreview.payment}
          />{#if $session.recruitPreview.moodWillDecrease && city && city.activationMood !== city.mood}<span
              class="activation-inline"
              ><strong>{city.mood} → {city.activationMood}</strong> after activation</span
            >{/if}{/if}
      </div>
      {#if ($session.recruitPreview?.payments?.length ?? 0) > 1}<label class="build-payment"
          >Pay with
          <select
            aria-label="Recruitment payment"
            value={JSON.stringify($session.recruitPreview!.payment)}
            onchange={(e) => controller.setRecruits($session.recruits, JSON.parse(e.currentTarget.value))}
          >
            {#each $session.recruitPreview!.payments! as payment}<option value={JSON.stringify(payment)}
                >{pileText(payment)}</option
              >{/each}
          </select>
        </label>{/if}
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
