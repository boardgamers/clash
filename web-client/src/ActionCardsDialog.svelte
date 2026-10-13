<script lang="ts">
  import { printedCardName, printedCardText } from './card-names';
  import { X, Layers, Zap, Swords, Archive, ScrollText, ChevronDown } from 'lucide-svelte';
  import ResourceText from './ResourceText.svelte';
  import ResourceAmount from './ResourceAmount.svelte';
  import type { Controller } from './controller';
  import { actionReason } from './model';
  import { handCardDescription } from './card-text';
  let { controller }: { controller: Controller } = $props();
  const session = $derived(controller.session);
  let tab = $state<'hand' | 'actions' | 'events'>('hand');
  const actions = $derived([...($session.game?.action_cards_discarded ?? [])].reverse().map(id => ({ id, card: $session.view?.cardCatalog?.find(card => card.id === id) })));
  const events = $derived([...($session.game?.incidents_discarded ?? [])].reverse().map(id => ({ id, card: $session.view?.eventCatalog?.find(card => card.id === id) })));

  const close = () => controller.patch({ cardsOpen: false });
  function show(node: HTMLDialogElement) {
    node.showModal();
  }
</script>

<dialog
  class="field-guide cards-dialog"
  aria-labelledby="action-cards-title"
  use:show
  onclose={close}
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
>
  <button class="close-guide icon-button" aria-label="Close action cards" onclick={close}
    ><X size={20} /></button
  >
  <h2 id="action-cards-title"><Layers size={23} />Cards</h2>
  <nav class="card-tabs" aria-label="Card piles">
    <button class:chosen={tab === 'hand'} aria-pressed={tab === 'hand'} onclick={() => tab = 'hand'}><Layers size={16} />Hand <span>{$session.view?.actionCards?.length ?? 0}</span></button>
    <button class:chosen={tab === 'actions'} aria-pressed={tab === 'actions'} onclick={() => tab = 'actions'}><Archive size={16} />Discarded actions <span>{actions.length}</span></button>
    <button class:chosen={tab === 'events'} aria-pressed={tab === 'events'} onclick={() => tab = 'events'}><ScrollText size={16} />Discarded events <span>{events.length}</span></button>
  </nav>
  {#if tab === 'hand'}
  {#if $session.view?.actionCards?.some((card) => card.tactics)}
    <p class="card-use-rule">
      Choose one use, then discard. Battle uses require Tactics: one card per combat round.
    </p>
  {/if}
  {#each $session.view?.actionCards ?? [] as card}
    <article class="play-card">
      <header class="play-card-header">
        <h3>{printedCardName(card.name)}</h3>
        <button
          class="primary card-play-button"
          aria-label={`Play ${printedCardName(card.name)}`}
          title={printedCardText(card.reason ?? 'Play this card')}
          disabled={!card.action || $session.pending}
          onclick={() => card.action && controller.submit(card.action, card.cost)}
          ><span>Play</span>
          <span class="card-play-cost"
            >{#if card.cost && Object.values(card.cost).some(Boolean)}<ResourceAmount pile={card.cost} />{/if}
            <span
              aria-label={card.free ? 'Free action' : 'Costs 1 action'}
              title={card.free ? 'Free action' : 'Costs 1 action'}><Zap size={13} />{card.free ? 0 : 1}</span
            ></span
          ></button
        >
      </header>
      <p><ResourceText text={handCardDescription(card.description)} /></p>
      {#if actionReason(card.reason)}<small>{printedCardText(actionReason(card.reason) ?? '')}</small>{/if}
      {#if card.tactics}<section
          class="card-battle-use"
          aria-label={`Battle use: ${printedCardName(card.tactics.name)}`}
        >
          <div class="card-use-label"><Swords size={15} />Or · Battle use</div>
          <h4>{printedCardName(card.tactics.name)}</h4>
          <p><ResourceText text={card.tactics.description} /></p>
        </section>{/if}
    </article>
  {:else}<p>No action cards in hand.</p>{/each}
  {:else}
    <p class="pile-hint">Top of the pile first. Select a card to read its rules.</p>
    <div class="discard-list">
      {#key tab}
      {#each (tab === 'actions' ? actions : events) as entry, i}
        <details class="discard-card">
          <summary>
            <span class="pile-number">{i + 1}</span>
            <span class="discard-name"><strong>{entry.card ? printedCardName(entry.card.name) : `${tab === 'actions' ? 'Action' : 'Event'} card ${entry.id}`}</strong>
              {#if entry.card && 'tactics' in entry.card && entry.card.tactics}<small><Swords size={12} />{printedCardName(entry.card.tactics.name)}</small>{/if}
            </span>
            {#if i === 0}<span class="top-card">Top card</span>{/if}
            <ChevronDown size={16} />
          </summary>
          <div class="discard-rules">
            <p class="discard-id">{tab === 'actions' ? 'Action' : 'Event'} · {tab === 'actions' ? 'A' : 'E'}{entry.id}</p>
            {#if entry.card && 'description' in entry.card}
              <p><ResourceText text={handCardDescription(entry.card.description)} /></p>
              {#if entry.card.tactics}<section class="card-battle-use">
                <div class="card-use-label"><Swords size={15} />Battle use</div>
                <h4>{printedCardName(entry.card.tactics.name)}</h4>
                <p><ResourceText text={entry.card.tactics.description} /></p>
              </section>{/if}
            {:else if entry.card && 'rules' in entry.card}
              {#each entry.card.rules as rule}<p><ResourceText text={rule} /></p>{/each}
            {:else}<p>Card details unavailable.</p>{/if}
          </div>
        </details>
      {:else}<p class="empty-pile">No discarded {tab === 'actions' ? 'action' : 'event'} cards.</p>{/each}
      {/key}
    </div>
  {/if}
  {#if $session.error}<p class="inline-error" role="alert">{$session.error}</p>{/if}
</dialog>

<style>
  .card-tabs { display: flex; gap: 0.4rem; flex-wrap: wrap; border-bottom: 1px solid var(--line); padding-bottom: 0.75rem; margin: 0.5rem 0 1rem; }
  .card-tabs button { display: inline-flex; align-items: center; gap: 0.4rem; border: 1px solid transparent; border-radius: 5px; background: transparent; padding: 0.6rem 0.7rem; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .card-tabs button.chosen { background: var(--paper2); color: var(--ink); border-color: var(--gold, #b79351); }
  .card-tabs button > span { font-size: 11px; padding: 0.1rem 0.35rem; border-radius: 4px; background: var(--paper2); color: var(--ink); }
  .pile-hint, .empty-pile { color: var(--muted); font-size: 13px; }
  .discard-list { display: grid; gap: 0.6rem; }
  .discard-card { background: var(--paper2); border: 1px solid var(--line); border-radius: 6px; overflow: hidden; }
  .discard-card summary { display: flex; align-items: center; gap: 0.8rem; cursor: pointer; list-style: none; padding: 0.8rem; }
  .discard-card summary::-webkit-details-marker { display: none; }
  .pile-number { color: var(--muted); font-size: 12px; min-width: 1rem; }
  .discard-name { display: grid; gap: 0.3rem; flex: 1; }
  .discard-name strong { font-size: 14px; }
  .discard-name small { display: flex; align-items: center; gap: 0.3rem; color: var(--muted); font-size: 12px; }
  .top-card { color: var(--ink); border: 1px solid var(--line); border-radius: 4px; padding: 0.2rem 0.4rem; font-size: 10px; white-space: nowrap; }
  .discard-card[open] summary { border-bottom: 1px solid var(--line); }
  .discard-card[open] summary > :global(svg:last-child) { transform: rotate(180deg); }
  .discard-rules { padding: 0.2rem 1rem 1rem; font-size: 14px; line-height: 1.6; }
  .discard-id { color: var(--muted); font-size: 11px; }
  .discard-rules .card-battle-use { margin-bottom: 0; }
  @media (max-width: 600px) { .card-tabs { gap: 0.15rem; } .card-tabs button { padding: 0.5rem 0.35rem; font-size: 11px; gap: 0.25rem; } }
</style>
