import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { handCardDescription } from './card-text.ts';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const view = (state: string, seat?: number): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
const move = (state: string, action: unknown, seat = engine.currentPlayer(state)): string =>
  engine.tryMove(state, JSON.stringify(action), seat);

async function calendar(event = 'Great Merchant', culture = 2) {
  let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'calendar-preview', {});
  state = move(state, { ChooseCivilization: 'Maya' });
  state = move(state, { ChooseCivilization: 'Rome' });
  const seat = engine.currentPlayer(state);
  const game = JSON.parse(state);
  const player = game.players[seat];
  player.advances = [...new Set([...player.advances, 'Fishing', 'Navigation', 'Cartography', 'Astronomy'])];
  player.resources.culture_tokens = culture;
  const eventId = view(state).eventCatalog!.find((card) => card.name === event)!.id;
  game.incidents_left = [eventId, ...game.incidents_left.filter((id: number) => id !== eventId)];
  state = JSON.stringify(game);
  const action = view(state, seat).specialActions!.find((action) => action.name === 'Calendar')!.action;
  return { state: move(state, action), seat, eventId, actionsLeft: game.actions_left };
}

function choose(state: string, seat: number, payment: object) {
  const { action } = JSON.parse(engine.webQuery(engine.stripSecret(state, seat), seat,
    JSON.stringify({ kind: 'decision', values: [], payments: [payment] })));
  assert.deepEqual(action, { Response: { Payment: [payment] } });
  return move(state, action, seat);
}

test('Calendar structures Great Merchant event, acquisition and civil effect without triggering the event', async () => {
  const { state, seat } = await calendar();
  const decision = view(state, seat).decision!;
  const preview = decision.calendarPreview!;
  assert.equal(decision.name, 'Calendar');
  assert.equal(preview.name, 'Great Merchant');
  assert.deepEqual(preview.rules, ['Barbarians move.']);
  assert.match(preview.card!.description, /^You may take the Event Card/);
  assert.match(handCardDescription(preview.card!.description), /^You may advance/);
  assert.match(handCardDescription(preview.card!.description), /Trade Routes income/);
  assert.equal(preview.card!.free, false);
  assert.equal(decision.eventContext, null, 'this is a preview, not a triggered event/card offer');
  assert.equal(JSON.parse(state).events.length, 1);
  assert.equal(JSON.parse(state).players[seat].resources.culture_tokens, 2);
  assert.deepEqual(decision.fields[0].choices, [{ culture_tokens: 1 }, {}]);
  // Every displayed rule comes from the already-revealed request, not deck order.
  for (const text of [...preview.rules, preview.card!.description]) {
    assert.ok(decision.fields[0].name.includes(text));
  }
  const reordered = JSON.parse(engine.stripSecret(state, seat));
  reordered.incidents_left.reverse();
  assert.deepEqual(JSON.parse(engine.webView(JSON.stringify(reordered), seat)).decision.calendarPreview, preview);
});

test('Calendar preview is private to its inspecting owner, including already-redacted requests', async () => {
  const { state, seat } = await calendar();
  for (const observer of [undefined, 1 - seat]) {
    const stripped = engine.stripSecret(state, observer);
    assert.ok(!view(state, observer).decision);
    assert.match(JSON.stringify(JSON.parse(stripped).events), /Calendar · Inspecting the next event/);
    assert.doesNotMatch(JSON.stringify(JSON.parse(stripped).events), /Great Merchant|Trade Routes income/);
    // Even asking for the owner's view of an observer-stripped state cannot
    // reconstruct the hidden event from the public catalog or shuffled deck.
    assert.equal(JSON.parse(engine.webView(stripped, seat)).decision.calendarPreview, null);
  }
});

test('Calendar top/bottom responses preserve payment, deck order, free-action and reinspection flow', async () => {
  const { state, seat, eventId, actionsLeft } = await calendar();
  const initial = JSON.parse(state);
  const kept = JSON.parse(choose(state, seat, {}));
  assert.deepEqual(kept.incidents_left, initial.incidents_left);
  assert.equal(kept.players[seat].resources.culture_tokens, 2);
  assert.equal(kept.actions_left, actionsLeft);
  const buried = JSON.parse(choose(state, seat, { culture_tokens: 1 }));
  assert.deepEqual(buried.incidents_left, [...initial.incidents_left.slice(1), eventId]);
  assert.equal(buried.players[seat].resources.culture_tokens, 1);
  assert.equal(buried.actions_left, actionsLeft);
  assert.equal(buried.events?.length ?? 0, 0);
  assert.ok(view(JSON.stringify(buried), seat).specialActions!.some((action) => action.name === 'Calendar' && action.action));
});

test('Calendar keeps the Great Person preview when unaffordable, with only the free keep response', async () => {
  const { state, seat } = await calendar('Great Merchant', 0);
  const decision = view(state, seat).decision!;
  assert.equal(decision.calendarPreview!.name, 'Great Merchant');
  assert.deepEqual(decision.fields[0].choices, [{}]);
  const kept = JSON.parse(choose(state, seat, {}));
  assert.equal(kept.players[seat].resources.culture_tokens ?? 0, 0);
});

test('Calendar distinguishes public/free Great Persons and leaves ordinary event UI unchanged', async () => {
  const { state, seat } = await calendar('Great Diplomat');
  const preview = view(state, seat).decision!.calendarPreview!;
  assert.match(preview.card!.description, /^You make place/);
  assert.match(handCardDescription(preview.card!.description), /^Choose another player\./);
  const freeCard = await calendar('Great Architect');
  assert.equal(view(freeCard.state, freeCard.seat).decision!.calendarPreview!.card!.free, true);
  const ordinary = await calendar('Famine');
  const decision = view(ordinary.state, ordinary.seat).decision!;
  assert.equal(decision.calendarPreview, null);
  assert.match(decision.fields[0].name, /^Famine · Barbarians move\./);
  assert.deepEqual(decision.fields[0].choices, [{ culture_tokens: 1 }, {}]);
});
