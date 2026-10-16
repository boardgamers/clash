<script lang="ts">
  import { printedCardName } from './card-names';
  import { onDestroy } from 'svelte';
  import { untrack } from 'svelte';
  import { Check, Sparkles, Layers, BookOpen, ChevronRight, Zap, Wheat } from 'lucide-svelte';
  import type { Controller } from './controller';
  import type { Decision, Move, Pile, UnitKind } from './types';
  import { unitInfo } from './city';
  import { resourceNames } from './types';
  import ResourceAmount from './ResourceAmount.svelte';
  import InfluenceFlow from './InfluenceFlow.svelte';
  import PaymentPicker from './PaymentPicker.svelte';
  import { samePayment } from './payment-options';
  import ResourceText from './ResourceText.svelte';
  import TerrainIcon from './TerrainIcon.svelte';
  import DecisionOptionContent from './DecisionOptionContent.svelte';
  import TacticsOption from './TacticsOption.svelte';
  import UnitPicker, { type UnitChoice } from './UnitPicker.svelte';
  import StructurePicker, { type StructureChoice } from './StructurePicker.svelte';
  import { researchPresentation } from './research';
  import { canCancelAbility, mapDecisionOptions } from './decision-controls';
  import { steelWeaponsBenefit } from './active-combat';
  let {
    controller,
    decision,
    onHighlight,
  }: { controller: Controller; decision: Decision; onHighlight: (position: string | null) => void } =
    $props();
  const session = $derived(controller.session);
  const panelId = $props.id();
  const selected = $derived($session.decisionSelection);
  const mapChoice = $derived(mapDecisionOptions(decision).length > 0);
  const combatBenefit = $derived(
    decision.name === 'Steel Weapons'
      ? steelWeaponsBenefit($session.game, $session.view, $session.view?.activePlayer ?? $session.seat)
      : null,
  );
  const description = $derived(
    combatBenefit ??
      (decision.tacticsSelection && decision.name === 'Tactics'
        ? 'Play one battle effect or skip.'
        : decision.name === 'Place Settler'
          ? 'Free Settler after losing a city.'
          : decision.description),
  );
  const pieceChoice = $derived(mapChoice && decision.options.some((option) => option.mapTarget));
  const unitChoice = $derived(pieceChoice && decision.options.every((o) => o.mapTarget?.kind === 'unit'));
  const structureChoices: StructureChoice[] = $derived(
    pieceChoice
      ? decision.options.flatMap((option, id) =>
          option.mapTarget?.kind === 'structure' && option.position
            ? [{ id, position: option.position, structure: option.mapTarget.structure }]
            : [],
        )
      : [],
  );
  const structureChoice = $derived(
    structureChoices.length > 0 && structureChoices.length === decision.options.length,
  );
  const unitChoices: UnitChoice[] = $derived(
    decision.options.flatMap((option, id) => {
      const target = option.mapTarget;
      if (target?.kind !== 'unit' || !option.position) return [];
      const units = $session.game?.players.find((p) => p.id === target.player)?.units;
      const unit = units?.find((u) => u.id === target.unit);
      const carrier =
        unit?.carrier_id ?? units?.find((u) => u.carried_units?.some((p) => p.id === target.unit))?.id;
      const name =
        typeof target.unitType === 'string'
          ? target.unitType
          : ($session.view?.players
              .find((p) => p.index === target.player)
              ?.leaders?.find((l) => l.unit === target.unit)?.name ?? target.unitType.Leader);
      return [
        {
          id,
          type: target.unitType,
          player: target.player,
          position: option.position,
          name,
          detail: carrier != null ? 'Aboard ship' : undefined,
          pirate: unit?.pirate,
          civilization: $session.game?.players.find((p) => p.id === target.player)?.civilization,
        },
      ];
    }),
  );
  let payments = $state<Pile[]>(
    untrack(() => decision.fields.map((f) => ({ ...(f.choices?.length === 1 ? f.choices[0] : f.initial) }))),
  );
  const directPayments = $derived.by(() => {
    const field = decision.fields.length === 1 && !decision.options.length ? decision.fields[0] : undefined;
    if (!field?.choices) return null;
    return field.choices.map((payment) => {
      try {
        return {
          payment,
          ...controller.query<{ action: Move }>({ kind: 'decision', values: [], payments: [payment] }),
        };
      } catch {
        return { payment, action: null };
      }
    });
  });
  function choosePayment(i: number, payment: Pile) {
    payments = payments.map((p, j) => (i === j ? payment : p));
  }
  const preview = $derived.by(() => {
    try {
      return {
        ...controller.query<{ action: Move }>({
          kind: 'decision',
          values: selected.map((i) => decision.options[i].value),
          payments,
        }),
        error: '',
      };
    } catch (error) {
      return { action: null, error: String(error) };
    }
  });
  const count = $derived(
    decision.min === decision.max ? `Choose ${decision.max}` : `Choose ${decision.min}–${decision.max}`,
  );
  const optionGroups = $derived.by(() => {
    const indexed = decision.options.map((option, index) => ({option,index}));
    if (new Set(indexed.map(({option})=>option.owner?.player).filter(p=>p!==undefined)).size < 2)
      return [{label:'', options:indexed}];
    const type = (option: typeof decision.options[number]) => option.card?.kind === 'action' ? 'Action cards' : option.card?.kind === 'objective' ? 'Objectives' : 'Wonders';
    const groups = new Map<string, {label:string; options:typeof indexed; order:number}>();
    for (const item of indexed) {
      const owner=item.option.owner, kind=type(item.option);
      const key=`${owner?.player}:${kind}`;
      const group=groups.get(key) ?? {label:`${owner?.civilization ?? 'Cards'}${owner?.you ? ' · You' : ''} — ${kind}`, options:[], order:(owner?.you ? 0 : (owner?.player ?? 0)+1)*10+['Action cards','Objectives','Wonders'].indexOf(kind)};
      group.options.push(item);groups.set(key,group);
    }
    return [...groups.values()].sort((a,b)=>a.order-b.order);
  });
  function toggle(i: number) {
    controller.selectDecisionOption(i);
  }
  onDestroy(() => onHighlight(null));
</script>

{#snippet options()}
  <div class="decision-options">
    {#each optionGroups as group}
      {#if group.label}<h3 class="decision-card-group">{group.label} <small>({group.options.length})</small></h3>{/if}
      {#each group.options as {option, index: i}}
      {@const unit =
        typeof option.value === 'string' && Object.hasOwn(unitInfo, option.value)
          ? unitInfo[option.value as UnitKind]
          : null}
      {@const presented = unit
        ? {
            ...option,
            name: option.value as string,
            description: option.description || unit.choiceEffect || unit.effect,
          }
        : option}
      {@const advance =
        typeof option.value === 'string'
          ? $session.view?.advances.find((a) => a.id === option.value)
          : undefined}
      {@const foodRecipient = decision.description === 'Select a player to gain 1 food'
        ? $session.game?.players.find((player) => player.id === option.value)
        : undefined}
      {@const Icon = unit?.icon ?? (advance ? researchPresentation(advance).icon : null)}
      <button
        class="decision-option"
        class:has-description={!!presented.description}
        class:card-option={!!option.card}
        class:selected={selected.includes(i)}
        aria-label={presented.name}
        aria-describedby={presented.description ? `${panelId}-option-${i}` : undefined}
        aria-pressed={selected.includes(i)}
        disabled={$session.pending ||
          (!selected.includes(i) && decision.max > 1 && selected.length >= decision.max)}
        onmouseenter={() => onHighlight(option.position)}
        onmouseleave={() => onHighlight(null)}
        onfocus={() => onHighlight(option.position)}
        onblur={() => onHighlight(null)}
        onclick={() => toggle(i)}
      >
        {#if option.terrain}<TerrainIcon terrain={option.terrain} />{:else if Icon}<Icon size={19} />{/if}
        <DecisionOptionContent option={presented} id={`${panelId}-option-${i}`} />
        {#if foodRecipient}
          <span class="recipient-food" aria-label="Food">
            {foodRecipient.resources?.food ?? 0}/{foodRecipient.resource_limit?.food ?? 0}<Wheat size={15} />
          </span>
        {/if}
        <span class="decision-selection" aria-hidden="true"
          >{#if selected.includes(i)}<Check size={13} />{/if}</span
        >
      </button>
      {/each}
    {/each}
  </div>
{/snippet}

<section
  class="action-panel floating-panel decision-panel"
  data-tutorial="decision"
  class:board-decision={mapChoice}
  class:selection-tray={mapChoice}
  class:unit-decision={unitChoice}
  class:card-decision={decision.options.some((option) => option.card)}
  class:tactics-decision={decision.tacticsSelection}
  aria-label={printedCardName(decision.name)}
>
  {#if decision.endOfAge}<span class="tiny-label">END OF AGE</span>{/if}
  <header class="decision-heading">
    {#if canCancelAbility($session.game, $session.view)}
      <button disabled={$session.pending} onclick={() => controller.submit('Undo')}>Cancel</button>
    {:else if $session.view?.canUndo && $session.game?.events?.some((event) => typeof event.event_type === 'object' && event.event_type !== null && 'CustomAction' in event.event_type)}
      <button disabled={$session.pending} onclick={() => controller.submit('Undo')}>Back</button>
    {/if}
    {#if $session.view?.influenceContext}
      <InfluenceFlow context={$session.view.influenceContext} />
    {:else}<h2><Sparkles size={21} />{printedCardName(decision.name)}</h2>{/if}
    {#if decision.eventContext}
      {@const context = decision.eventContext}
      <div class="decision-event-context" class:full-context={!!context.card}>
        {#if context.card}
          {@const card = context.card}
          <div class="event-card-offer">
            <Layers size={16} />
            <span
              >{card.later
                ? card.firstOffer
                  ? 'Then: take the card'
                  : 'If passed to you'
                : 'Take the card'}</span
            >
            <ResourceAmount pile={card.cost} />
          </div>
          <details class="event-rules">
            <summary aria-label={`${printedCardName(card.name)} event details`}
              ><BookOpen size={13} />{printedCardName(card.name)}<ChevronRight size={13} /></summary
            >
            {#if context.placement}<p>{context.placement}</p>{/if}
            {#if context.raid}<p class="event-raid"><ResourceText text={context.raid} /></p>{/if}
            <p>
              The player who triggered the event may take the card for <ResourceAmount
                pile={{ culture_tokens: 1 }}
              />. If they pass, other players may take it in turn order for <ResourceAmount
                pile={{ culture_tokens: 2 }}
              />.
            </p>
            <p>
              <span class="event-card-use">Play <Zap size={13} />{card.free ? 0 : 1}</span><ResourceText
                text={card.description}
              />
            </p>
          </details>
        {:else}
          <details class="event-rules">
            <summary aria-label={`${printedCardName(context.name)} event details`}
              ><BookOpen size={13} />Event rules<ChevronRight size={13} /></summary
            >
            {#if context.placement}<p>{context.placement}</p>{/if}
            {#if context.raid}<p class="event-raid"><ResourceText text={context.raid} /></p>{/if}
            {#each context.rules as rule}<p><ResourceText text={rule} /></p>{/each}
          </details>
        {/if}
      </div>
    {/if}
  </header>
  {#if description && !$session.view?.influenceContext}<p class="decision-description">
      <ResourceText
        text={description.replaceAll('Status Phase', 'End of age').replaceAll('status phase', 'end of age')}
      />
    </p>{/if}
  {#if decision.options.length}
    {#if decision.tacticsSelection}
      <div class="decision-options tactics-options">
        {#each decision.options as option, i}
          <TacticsOption
            {option}
            selected={selected.includes(i)}
            pending={$session.pending}
            id={`${panelId}-option-${i}`}
            onSelect={() => toggle(i)}
          />
        {/each}
      </div>
    {:else if unitChoice}
      <UnitPicker
        choices={unitChoices}
        {selected}
        position={$session.decisionPosition ?? null}
        limit={decision.max}
        pending={$session.pending}
        colorBlind={$session.colorBlind}
        playerColors={$session.playerColors}
        label={count}
        onPosition={(position) => controller.focusDecisionPosition(position)}
        onSelect={toggle}
        {onHighlight}
      />
    {:else if structureChoice}
      <StructurePicker
        choices={structureChoices}
        {selected}
        position={$session.decisionPosition ?? null}
        limit={decision.max}
        pending={$session.pending}
        onPosition={(position) => controller.focusDecisionPosition(position)}
        onSelect={toggle}
        {onHighlight}
      />
    {:else if pieceChoice}
      {@render options()}
    {:else if !mapChoice}
      {@render options()}
    {/if}
  {/if}
  {#if directPayments}
    {@const field = decision.fields[0]}
    {@const noResourceCost =
      !decision.reward &&
      !field.optional &&
      directPayments.length > 0 &&
      directPayments.every((choice) => !Object.values(choice.payment).some(Boolean))}
    {#if field.name !== decision.name && !combatBenefit && !$session.view?.influenceContext}<p
        class="decision-description"
      >
        <ResourceText text={noResourceCost ? 'Free' : field.name} />
      </p>{/if}
    {#if $session.view?.influenceContext}
      {@const context = $session.view.influenceContext}
      <div class="influence-decisions">
        {#each directPayments as choice}
          {@const paid = Object.values(choice.payment).some(Boolean)}
          <button
            class:primary={paid}
            class:secondary={!paid}
            disabled={!choice.action || $session.pending}
            onclick={() => choice.action && controller.submit(choice.action)}
          >
            <span
              >{context.stage === 'boost'
                ? paid
                  ? 'Spend culture & succeed'
                  : 'Accept failure'
                : context.stage === 'range'
                  ? 'Extend range & roll'
                  : 'Pay & continue'}</span
            >
            {#if paid}<ResourceAmount pile={choice.payment} compact />{/if}
          </button>
        {/each}
      </div>
    {:else}
      <PaymentPicker
        options={directPayments.map((choice) => ({ payment: choice.payment, disabled: !choice.action }))}
        value={payments[0]}
        onChange={(payment) => choosePayment(0, payment)}
        onPay={(payment) => {
          const action = directPayments.find((choice) => samePayment(choice.payment, payment))?.action;
          if (action) controller.submit(action);
        }}
        pending={$session.pending}
        optional={field.optional}
        reward={decision.reward}
        variableAmount={field.variableAmount}
        label={field.name}
      />
    {/if}
  {:else}
    {#each decision.fields as field, i}
      <div class="decision-payment">
        {#if decision.fields.length > 1 || field.name !== decision.name}<strong
            ><ResourceText text={field.name} /></strong
          >{/if}
        {#if field.choices}
          <PaymentPicker
            options={field.choices.map((payment) => ({ payment }))}
            value={payments[i]}
            onChange={(payment) => choosePayment(i, payment)}
            pending={$session.pending}
            optional={field.optional}
            reward={decision.reward}
            variableAmount={field.variableAmount}
            label={field.name}
          />
        {:else}
          {#if !decision.reward}<div class="decision-cost">
              Cost <ResourceAmount pile={field.cost} />
            </div>{/if}
          <div class="payment-amounts">
            {#each field.resources as resource}
              {@const available = decision.reward
                ? 255
                : ($session.game?.players.find((p) => p.id === $session.seat)?.resources?.[resource] ?? 0)}
              <label
                ><span>{resourceNames[resource]}</span>
                <select
                  aria-label={`${resourceNames[resource]} amount for ${field.name}`}
                  disabled={$session.pending}
                  value={payments[i][resource] ?? 0}
                  onchange={(event) =>
                    choosePayment(i, { ...payments[i], [resource]: Number(event.currentTarget.value) })}
                >
                  {#each Array.from({ length: available + 1 }, (_, n) => n) as n}<option value={n}>{n}</option
                    >{/each}
                </select>
              </label>
            {/each}
          </div>
          {#if field.optional}<button
              class="secondary compact"
              disabled={$session.pending}
              onclick={() => choosePayment(i, {})}>Decline</button
            >{/if}
        {/if}
      </div>
    {/each}
    {#if !$session.analysis && decision.endOfAge && decision.name === 'Raze city' && decision.min === 0}
      <label class="skip-raze-preference">
        <input
          type="checkbox"
          checked={$session.skipRazeCity}
          disabled={$session.pending}
          onchange={(event) => controller.setSkipRazeCity(event.currentTarget.checked)}
        />
        Keep all cities; skip this step in future ages
      </label>
    {/if}
    <div class="decision-footer" class:map-decision-footer={mapChoice}>
      {#if decision.tacticsSelection}
        {#if selected.length}<small class="tactics-discard">Discards the selected card</small>{/if}
        <button
          class="secondary"
          disabled={$session.pending}
          onclick={() => {
            const response = controller.query<{ action: Move }>({
              kind: 'decision',
              values: [],
              payments: [],
            });
            controller.submit(response.action);
          }}>Skip</button
        >
        {#if selected.length}<button
            class="primary"
            disabled={!preview.action || $session.pending}
            onclick={() => preview.action && controller.submit(preview.action)}
            >Play tactics<Check size={16} /></button
          >{/if}
      {:else}
        {#if decision.options.length && !mapChoice}<div class="decision-count">
            {count}<span>{selected.length}/{decision.max}</span>
          </div>{/if}
        {#if mapChoice && !unitChoice}<span
            class="map-selection-count"
            aria-label={`${selected.length} of ${decision.max} selected`}
            >{selected.length}/{decision.max}</span
          >{/if}
        <button
          class="primary"
          class:wide={!mapChoice}
          aria-label={mapChoice && selected.length
            ? `Confirm · ${selected.length}/${decision.max}`
            : undefined}
          disabled={!preview.action || $session.pending}
          title={preview.error || 'Confirm selection'}
          onclick={() => preview.action && controller.submit(preview.action)}
        >
          {selected.length === 0 && !decision.fields.length && decision.min === 0 ? 'Skip' : 'Confirm'}<Check
            size={16}
          />
        </button>
      {/if}
      {#if preview.error && (selected.length > 0 || decision.fields.length)}<p class="inline-error">
          {preview.error}
        </p>{/if}
    </div>
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</section>

<style>
  .decision-card-group { grid-column: 1 / -1; margin: 1rem 0 .25rem; font-size: .95rem; padding-bottom: .5rem; border-bottom: 1px solid var(--line); }
  .decision-card-group:first-child { margin-top: 0; }
  .decision-card-group small { font-weight: normal; color: var(--muted); }

  .recipient-food {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
</style>
