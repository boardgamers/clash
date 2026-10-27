import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'early-choices', {})).players.slice(2);
function fixture(name: string) {
  const g = JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
  g.players = [...g.players.slice(0, 2), ...structuredClone(npcs)];
  return JSON.stringify(g);
}
const apply = (raw: string, move: unknown, player = engine.currentPlayer(raw)) => engine.tryMove(raw, JSON.stringify(move), player);
const view = (raw: string, player: number) => JSON.parse(engine.webView(engine.stripSecret(raw, player), player));
const queue = (raw: string, player: number, key: string, response: unknown) => apply(raw, { QueueChoice: { key, response } }, player);
const actions = (raw: string) => JSON.parse(raw).log.flatMap((a: any) => a.rounds.flatMap((r: any) => r.turns.flatMap((t: any) => t.actions ?? [])));
function research() { return apply(fixture('status_phase/free_advance'), { Playing: 'EndTurn' }); }

test('early research is private, editable and persisted without spending or advancing play', () => {
  const before = research();
  assert.equal(engine.currentPlayer(before), 1);
  const offer = view(before, 0).earlyChoices[0];
  const selected = offer.options.find((o: any) => o.response.SelectAdvance === 'Philosophy');
  assert.ok(selected);
  const command = JSON.stringify({ QueueChoice: { key: offer.key, response: selected.response } });
  assert.equal(engine.canMoveOutOfTurn(before, command, 0), true);
  const queued = queue(before, 0, offer.key, selected.response);
  assert.equal(engine.isLiveUpdate(queued), true);
  assert.equal(engine.currentPlayer(queued), 1);
  assert.deepEqual(JSON.parse(queued).players, JSON.parse(before).players);
  assert.equal(actions(queued).length, actions(before).length);
  assert.equal(JSON.parse(engine.stripSecret(queued, 1)).early_choices, undefined);
  assert.equal(JSON.parse(engine.stripSecret(queued, undefined)).early_choices, undefined);
  assert.deepEqual(view(queued, 0).earlyChoices[0].selected, selected.response);
  const cancelled = queue(queued, 0, offer.key, null);
  assert.equal(JSON.parse(cancelled).early_choices, undefined);
  const edited = queue(queued, 0, offer.key, offer.options[0].response);
  assert.equal(JSON.parse(edited).early_choices.length, 1);
});

test('queued research resolves in order as a normal journal response and replays', () => {
  const before = research();
  const offer = view(before, 0).earlyChoices[0];
  const queued = queue(before, 0, offer.key, { SelectAdvance: 'Philosophy' });
  const after = apply(queued, { Response: { SelectAdvance: 'Storage' } }, 1);
  assert.ok(JSON.parse(after).players[0].advances.includes('Philosophy'));
  assert.ok(JSON.parse(after).players[1].advances.includes('Storage'));
  assert.equal(JSON.parse(after).early_choices, undefined);
  assert.equal(engine.isLiveUpdate(after), false);
  const added = actions(after).slice(actions(before).length).map((a: any) => a.action);
  assert.deepEqual(added.slice(0, 2), [{ Response: { SelectAdvance: 'Storage' } }, { Response: { SelectAdvance: 'Philosophy' } }]);
  let replay = before;
  for (const action of added) if (action !== 'StartTurn') replay = apply(replay, action);
  assert.deepEqual(JSON.parse(replay).players, JSON.parse(after).players);
});

test('forged, stale, active-player and spectator queues are rejected', () => {
  const raw = research();
  const offer = view(raw, 0).earlyChoices[0];
  for (const [player, key, response] of [
    [0, offer.key, { SelectAdvance: 'SteelWeapons' }],
    [0, 'old-window', offer.options[0].response],
    [1, offer.key, offer.options[0].response],
    [99, offer.key, offer.options[0].response],
  ] as const) assert.throws(() => queue(raw, player, key, response));
});

test('a queued raze invalidated by a board change asks again instead of choosing another city', () => {
  let raw = fixture('status_phase/raze_city');
  const offer = view(raw, 1).earlyChoices[0];
  const pick = offer.options.find((o: any) => o.response.SelectPositions.length);
  assert.ok(pick);
  raw = queue(raw, 1, offer.key, pick.response);
  const changed = JSON.parse(raw);
  const city = changed.players[1].cities.find((c: any) => c.position === pick.response.SelectPositions[0]);
  city.city_pieces = { temple: 1 };
  raw = apply(JSON.stringify(changed), { Response: { SelectPositions: [] } }, 0);
  assert.ok(JSON.parse(raw).players[1].cities.some((c: any) => c.position === city.position));
  assert.equal(JSON.parse(raw).early_choices, undefined);
});

function person() {
  const g = JSON.parse(fixture('incidents/pirates_spawn'));
  g.incidents_left = [55];
  g.players[1].resources = { food: 2, culture_tokens: 4 };
  let raw = JSON.stringify(g);
  for (const response of [{ SelectAdvance: 'Storage' }, { SelectPositions: ['A2'] }, { SelectPositions: ['D1'] }, { Payment: [{ food: 1 }] }]) raw = apply(raw, { Response: response });
  return raw;
}

test('Great Person preacceptance respects priority and waits through the recipient pirate payment', () => {
  const raw = person();
  const offer = view(raw, 1).earlyChoices[0];
  assert.ok(offer);
  const queued = queue(raw, 1, offer.key, { Payment: [{ culture_tokens: 2 }] });
  const passed = apply(queued, { Response: { Payment: [{}] } }, 0);
  assert.equal(engine.currentPlayer(passed), 1);
  assert.ok(JSON.parse(passed).events.at(-1).handler.request.Payment[0].name.includes('pirates'));
  assert.ok(!(JSON.parse(passed).players[1].action_cards ?? []).includes(155));
  const after = apply(passed, { Response: { Payment: [{ food: 1 }] } }, 1);
  assert.ok(JSON.parse(after).players[1].action_cards.includes(155));
  assert.equal(JSON.parse(after).players[1].resources.culture_tokens, 2);
});

test('Great Person claimed by earlier player expires conditional offers without charging others', () => {
  const raw = person();
  const offer = view(raw, 1).earlyChoices[0];
  const queued = queue(raw, 1, offer.key, { Payment: [{ culture_tokens: 2 }] });
  let after = apply(queued, { Response: { Payment: [{ culture_tokens: 1 }] } }, 0);
  after = apply(after, { Response: { Payment: [{ food: 1 }] } }, 1);
  assert.equal(JSON.parse(after).players[1].resources.culture_tokens, 4);
  assert.ok(JSON.parse(after).players[0].action_cards.includes(155));
  assert.equal(JSON.parse(after).early_choices, undefined);
});

test('early objective choices stay private and complete only when that player is reached', () => {
  const g = JSON.parse(fixture('objective_cards/status_phase/standing_army'));
  g.players[1].objective_cards = [13];
  g.players[1].resources.wood = 7;
  let raw = apply(JSON.stringify(g), { Playing: 'EndTurn' }, 1);
  const offer = view(raw, 1).earlyChoices.find((o: any) => o.title === 'Wood Supplies');
  assert.ok(offer, 'other player can queue a qualifying status objective');
  const pick = offer.options.find((o: any) => o.response.SelectHandCards.length);
  for (const other of view(raw, 1).earlyChoices) {
    raw = queue(raw, 1, other.key, other.key === offer.key ? pick.response : { SelectHandCards: [] });
  }
  assert.ok(!JSON.parse(raw).players[1].completed_objectives?.length);
  for (let i = 0; i < 3 && engine.currentPlayer(raw) === 0; i++) {
    if (!JSON.parse(raw).events.at(-1)?.handler?.request.SelectHandCards) break;
    raw = apply(raw, { Response: { SelectHandCards: [] } }, 0);
  }
  assert.ok(JSON.parse(raw).players[1].completed_objectives.some((o: any) => o.name === 'Wood Supplies'));
});

test('objectives can be queued before status, replace the other side, and expire when no longer met', () => {
  const g = JSON.parse(fixture('objective_cards/status_phase/standing_army'));
  g.players[1].objective_cards = [13];
  g.players[1].resources.wood = 7;
  const before = JSON.stringify(g);
  const offers = view(before, 1).earlyChoices;
  const wood = offers.find((o: any) => o.title === 'Wood Supplies');
  assert.ok(wood, 'objective is available before ending the turn');
  const response = { SelectHandCards: [{ ObjectiveCard: 13 }] };
  const queued = queue(before, 1, wood.key, response);
  assert.equal(JSON.parse(queued).early_choices.length, 1);
  const other = offers.find((o: any) => o.key !== wood.key && o.options.some((v: any) => JSON.stringify(v.response) === JSON.stringify(response)));
  if (other) {
    const replaced = queue(queued, 1, other.key, response);
    assert.equal(JSON.parse(replaced).early_choices.length, 1);
    assert.equal(JSON.parse(replaced).early_choices[0].key, other.key);
  }
  const changed = JSON.parse(queued);
  changed.players[1].resources.wood = 0;
  const after = apply(JSON.stringify(changed), { Playing: 'EndTurn' }, 1);
  assert.ok(!(JSON.parse(after).early_choices ?? []).some((c: any) => c.key === wood.key));
});

test('a pre-status objective queue resolves only at that player’s objective check', () => {
  const g = JSON.parse(fixture('objective_cards/status_phase/standing_army'));
  g.players[1].objective_cards = [13];
  g.players[1].resources.wood = 7;
  let raw = JSON.stringify(g);
  for (const offer of view(raw, 1).earlyChoices) {
    raw = queue(raw, 1, offer.key, offer.title === 'Wood Supplies'
      ? { SelectHandCards: [{ ObjectiveCard: 13 }] } : { SelectHandCards: [] });
  }
  raw = apply(raw, { Playing: 'EndTurn' }, 1);
  assert.ok(!JSON.parse(raw).players[1].completed_objectives?.length);
  for (let i = 0; i < 3 && engine.currentPlayer(raw) === 0; i++) {
    if (!JSON.parse(raw).events.at(-1)?.handler?.request.SelectHandCards) break;
    raw = apply(raw, { Response: { SelectHandCards: [] } }, 0);
  }
  assert.ok(JSON.parse(raw).players[1].completed_objectives.some((o: any) => o.name === 'Wood Supplies'));
});

function governmentQueueSetup() {
  const g = JSON.parse(fixture('status_phase/change_government'));
  g.players[1].advances = [...g.players[0].advances];
  g.players[0].resources.mood_tokens = 6;
  return apply(JSON.stringify(g), {Response:{SelectPositions:[]}});
}

test('first player can be queued during government choices and resolves only for the actual chooser', () => {
  const before = governmentQueueSetup();
  const offer = view(before, 1).earlyChoices.find((o: any) => o.key.endsWith(':first'));
  assert.ok(offer);
  let state = queue(before, 1, offer.key, {SelectPlayer:1});
  state = engine.setPlayerSettings(state, 1, {keepGovernment:true});
  state = apply(state, {Response:{Payment:[{}]}});
  assert.equal(JSON.parse(state).starting_player_index, 1);
  assert.equal(JSON.parse(state).early_choices, undefined);
});

test('a changed first-player chooser does not consume another player’s queued selection', () => {
  const before = governmentQueueSetup();
  const offer = view(before, 1).earlyChoices.find((o:any) => o.key.endsWith(':first'));
  let state = queue(before,1,offer.key,{SelectPlayer:1});
  const g = JSON.parse(state);
  g.players[1].resources.mood_tokens = 5;
  state = engine.setPlayerSettings(JSON.stringify(g), 1, {keepGovernment:true});
  state = apply(state,{Response:{Payment:[{}]}});
  assert.equal(engine.currentPlayer(state),0);
  assert.deepEqual(JSON.parse(state).events.at(-1).handler.request.SelectPlayer.choices, [0,1]);
  assert.equal(JSON.parse(state).starting_player_index, 0);
});

test('first-player queue is visible only to the current eligible chooser, including ties', () => {
  const g = JSON.parse(governmentQueueSetup());
  const hasOffer = () => view(JSON.stringify(g), 1).earlyChoices.some((o: any) => o.key.endsWith(':first'));
  assert.equal(hasOffer(), true); // Greece: 14 tokens, Rome: 13.
  g.players[1].resources.mood_tokens = 6;
  assert.equal(hasOffer(), false); // Tie goes to Rome, first in turn order.
  g.players[1].resources.mood_tokens = 5;
  assert.equal(hasOffer(), false);
});

test('keep-government preference skips the optional payment without changing advances or resources', () => {
  const before = governmentQueueSetup();
  const queued = engine.setPlayerSettings(before, 1, {keepGovernment:true});
  const after = apply(queued, {Response:{Payment:[{}]}});
  const p = JSON.parse(after).players[1];
  assert.deepEqual(p.advances, JSON.parse(before).players[1].advances);
  assert.deepEqual(p.resources, JSON.parse(before).players[1].resources);
  assert.ok(JSON.parse(after).events.at(-1).handler.request.SelectPlayer);
  assert.ok(!view(before,1).earlyChoices.some((o:any) => o.key.endsWith('/government')));
});

test('disabling keep-government restores the normal payment prompt', () => {
  let state = engine.setPlayerSettings(governmentQueueSetup(),1,{keepGovernment:true});
  state = engine.setPlayerSettings(state,1,{keepGovernment:false});
  state = apply(state,{Response:{Payment:[{}]}});
  assert.equal(engine.currentPlayer(state),1);
  assert.equal(JSON.parse(state).events.at(-1).handler.request.Payment[0].name,'Pay to change government');
});
