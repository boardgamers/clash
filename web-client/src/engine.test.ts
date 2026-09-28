import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createViewer } from '@boardgamers/protocol/viewer';
import { ChatController } from '@boardgamers/protocol/chat';
import { journal, positionXY } from './model.ts';
import type { View, Game } from './types.ts';
const require = createRequire(import.meta.url);
const engine = require('../.engine/server.js');
async function initial() {
  return engine.init(2, [], { undo: 'SamePlayer', civilization: 'Random' }, 'clash-preview-20260927', {});
}

test('opening collection, research, undo, and handoff use the Rust engine', async () => {
  let state = await initial();
  const seat = engine.currentPlayer(state);
  const publicState = engine.stripSecret(state, seat);
  const view: View = JSON.parse(engine.webView(publicState, seat));
  const city = view.cities[0];
  const choices = city.choices.filter((c) => c.pile.wood || c.pile.ore).map((c) => ({ ...c, times: 1 }));
  const preview = JSON.parse(
    engine.webCollectPreview(publicState, seat, city.position, JSON.stringify(choices)),
  );
  state = engine.tryMove(state, JSON.stringify(preview.action), seat);
  let game: Game = JSON.parse(state);
  assert.equal(game.actions_left, 2);
  assert.equal(game.players[seat].resources?.wood, 1);
  assert.equal(game.players[seat].resources?.ore, 1);
  const afterCollection = state;
  const research = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)).advances.find(
    (a: any) => a.id === 'Storage',
  );
  assert.ok(research.action);
  state = engine.tryMove(state, JSON.stringify(research.action), seat);
  game = JSON.parse(state);
  assert.equal(game.actions_left, 1);
  assert.equal(game.players[seat].resource_limit?.food, 7);
  state = engine.tryMove(state, '"Undo"', seat);
  game = JSON.parse(state);
  assert.equal(game.actions_left, 2);
  assert.equal(game.players[seat].resource_limit?.food, 2);
  assert.deepEqual(journal(game), journal(JSON.parse(afterCollection)));
  state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), seat);
  assert.notEqual(engine.currentPlayer(state), seat);
});

test('viewer queries respect spectator and opponent seats and reject illegal choices', async () => {
  const state = await initial();
  const seat = engine.currentPlayer(state);
  const visible = engine.stripSecret(state, seat);
  for (const index of [undefined, 99, 1 - seat]) {
    const view: View = JSON.parse(engine.webView(visible, index));
    assert.equal(view.canPlay, false);
    assert.equal(view.canUndo, false);
    assert.ok(view.advances.every((a) => a.action === null));
  }
  const view: View = JSON.parse(engine.webView(visible, seat));
  const city = view.cities[0];
  assert.throws(() => engine.webCollectPreview(visible, 1 - seat, city.position, '[]'));
  assert.throws(() =>
    engine.webCollectPreview(
      visible,
      seat,
      city.position,
      JSON.stringify([{ position: city.position, pile: { gold: 7 }, times: 1 }]),
    ),
  );
  const full = JSON.parse(state);
  const exhausted = { ...full, actions_left: 0 };
  const exhaustedView: View = JSON.parse(
    engine.webView(engine.stripSecret(JSON.stringify(exhausted), seat), seat),
  );
  assert.ok(exhaustedView.settlers.length > 0);
  assert.ok(exhaustedView.settlers.every((u) => !u.foundAction && u.destinations.length === 0));
  const limited = JSON.parse(visible);
  assert.ok(full.players[1 - seat].action_cards.some((id: number) => id !== 0));
  assert.ok(limited.players[1 - seat].action_cards.every((id: number) => id === 0));
});

test('objective cards show both conditions only from the player-visible hand', async () => {
  const state = await initial();
  const seat = engine.currentPlayer(state);
  const visible = engine.stripSecret(state, seat);
  const view: View = JSON.parse(engine.webView(visible, seat));
  const hand = JSON.parse(visible).players[seat].objective_cards;
  assert.deepEqual(
    view.objectiveCards.map((card) => card.id),
    hand,
  );
  assert.ok(view.objectiveCards.length > 0);
  for (const card of view.objectiveCards) {
    assert.equal(card.objectives.length, 2);
    for (const objective of card.objectives) {
      assert.ok(objective.name.length > 0);
      assert.ok(objective.description.length > 0);
      assert.ok(['Instant', 'Status phase'].includes(objective.timing));
    }
  }
  assert.deepEqual(
    view.objectiveCards[0].objectives.map((o) => o.name),
    ['Legacy', 'Ivory Tower'],
  );
  for (const index of [undefined, 99, 1 - seat]) {
    assert.deepEqual(JSON.parse(engine.webView(visible, index)).objectiveCards, []);
  }
  const spectatorState = engine.stripSecret(state, undefined);
  assert.deepEqual(JSON.parse(engine.webView(spectatorState, seat)).objectiveCards, []);
});

test('an eligible status objective can be claimed or kept, with correct points and privacy', async () => {
  const setup = JSON.parse(await initial());
  const seat = setup.current_player_index;
  // A city with an Obelisk makes the existing Legacy objective eligible.
  setup.players[seat].cities[0].city_pieces = { obelisk: seat };
  let state = JSON.stringify(setup);
  for (let turn = 0; turn < 6; turn++) {
    assert.equal(JSON.parse(state).events?.length ?? 0, 0);
    state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), engine.currentPlayer(state));
  }
  const visible = engine.stripSecret(state, seat);
  const view: View = JSON.parse(engine.webView(visible, seat));
  const decision = view.objectiveDecision!;
  assert.ok(decision);
  assert.equal(view.supportedPhase, true);
  assert.equal(view.canPlay, false);
  assert.equal(decision.name, 'Legacy');
  assert.equal(decision.points, 2);
  assert.equal(decision.cards.length, 1);
  assert.equal(decision.cards[0].id, 36);
  for (const index of [undefined, 1 - seat]) {
    const limited = engine.stripSecret(state, index);
    assert.equal(JSON.parse(engine.webView(limited, index)).objectiveDecision, null);
    // Switching a seat never reveals card choices from already-filtered data.
    assert.equal(JSON.parse(engine.webView(limited, seat)).objectiveDecision, null);
  }
  const claimed = engine.tryMove(state, JSON.stringify(decision.cards[0].action), seat);
  const claimedPlayer = JSON.parse(claimed).players[seat];
  assert.ok(!claimedPlayer.objective_cards?.includes(36));
  assert.ok(claimedPlayer.completed_objectives.some((o: any) => o.card === 36 && o.name === 'Legacy'));
  const after: View = JSON.parse(engine.webView(engine.stripSecret(claimed, seat), seat));
  assert.equal(
    after.players.find((p) => p.index === seat)!.score,
    view.players.find((p) => p.index === seat)!.score + 2,
  );
  assert.equal(after.objectiveDecision, null);
  assert.ok(journal(JSON.parse(claimed)).some((entry) => entry.text.includes('Legacy')));
  assert.ok(decision.skip);
  const kept = engine.tryMove(state, JSON.stringify(decision.skip), seat);
  assert.ok(JSON.parse(kept).players[seat].objective_cards.includes(36));
  const keptView: View = JSON.parse(engine.webView(engine.stripSecret(kept, seat), seat));
  assert.equal(keptView.objectiveDecision, null);
  assert.equal(
    keptView.players.find((p) => p.index === seat)!.score,
    view.players.find((p) => p.index === seat)!.score,
  );
});

test('research graph has valid prerequisites and setup summaries retain historical locations', async () => {
  const state = await initial();
  const seat = engine.currentPlayer(state);
  const game: Game = JSON.parse(engine.stripSecret(state, seat));
  const view: View = JSON.parse(engine.webView(JSON.stringify(game), seat));
  assert.equal(new Set(view.advances.map((a) => a.group)).size, 12);
  for (const advance of view.advances) {
    if (advance.required) assert.ok(view.advances.some((a) => a.id === advance.required));
    assert.ok(Number.isInteger(advance.order));
  }
  const storage = view.advances.find((a) => a.id === 'Storage')!;
  assert.equal(storage.required, 'Farming');
  assert.equal(storage.group, 'Agriculture');
  assert.deepEqual(storage.bonus, { mood_tokens: 1 });
  assert.equal(view.advances.find((a) => a.id === 'Voting')!.required, 'Philosophy');
  const before = journal(game).filter((entry) => entry.kind === 'setup');
  assert.equal(before.length, 2);
  assert.ok(
    before.every(
      (entry) => entry.setup?.position && !entry.text.includes('Gain') && !entry.text.includes('card'),
    ),
  );
  game.players.forEach((player) => {
    player.cities = [];
  });
  assert.deepEqual(
    journal(game).filter((entry) => entry.kind === 'setup'),
    before,
  );
});

test('BGS lifecycle, chat acknowledgements, journal, and cleanup', async () => {
  const chat = new ChatController({ readDelayMs: 0 });
  let renders = 0;
  let ready = 0;
  let lines: string[] = [];
  const client = createViewer<string, string>({
    chat,
    onPlayer() {},
    onState(state) {
      renders++;
      client.replaceLog(journal(JSON.parse(state)).map((l) => l.text));
    },
  });
  client.emitter.on('ready', () => ready++);
  client.emitter.on('replaceLog', (payload) => {
    lines = payload;
  });
  client.emitter.emit('player', { index: 1 });
  client.emitter.emit('state', engine.stripSecret(await initial(), 1));
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.equal(renders, 1);
  assert.equal(ready, 1);
  assert.ok(lines.length > 0);
  assert.equal(client.diagnostics(['chat']).compatible, true);
  client.emitter.emit('chat:state', { canSend: true, readState: { userId: 'me', lastReadAt: 0 } });
  client.emitter.on('chat:send', (payload) => {
    assert.equal(payload.text, 'hello');
    client.emitter.emit('chat:result', { requestId: payload.requestId!, ok: false, error: 'Try again' });
  });
  chat.setDraft('hello');
  chat.submit();
  assert.equal(chat.snapshot.draft, 'hello');
  assert.equal(chat.snapshot.error, 'Try again');
  client.emitter.emit('chat:appended', [
    {
      _id: '0123456789abcdef01234567',
      author: 'Aurelia',
      authorId: 'other',
      playerIndex: 0,
      text: 'Welcome',
      createdAt: new Date().toISOString(),
      type: 'text',
    },
  ]);
  assert.equal(chat.unread, 1);
  client.emitter.emit('chat:updated', [
    { _id: '0123456789abcdef01234567', text: 'Welcome back', type: 'text' },
  ]);
  assert.equal(chat.snapshot.messages[0].text, 'Welcome back');
  client.emitter.emit('chat:deleted', ['0123456789abcdef01234567']);
  assert.equal(chat.snapshot.messages.length, 0);
  client.destroy();
  assert.equal(client.emitter.emit('state', '{}'), false);
});

test('settlers move, found a second city, and unlock construction without exceeding the turn budget', async () => {
  const setup = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(setup));
  const p = setup.players[seat];
  p.resources = { food: 2, wood: 4, ore: 4, ideas: 4, mood_tokens: 4 };
  p.advances.push('Writing');
  let state = JSON.stringify(setup);
  const view = () => JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)) as View;
  const city = view().cities[0].position;
  assert.equal(view().cityActions[0].buildings.find((b) => b.name === 'Academy')!.reason, 'Need more cities');
  const destination = view().settlers[0].destinations.find((d) => d.terrain === 'Forest')!;
  assert.ok(destination);
  state = engine.tryMove(state, JSON.stringify(destination.action), seat);
  assert.equal(JSON.parse(state).actions_left, 2);
  if (view().stopMovement) state = engine.tryMove(state, JSON.stringify(view().stopMovement), seat);
  const found = view().settlers[0].foundAction;
  assert.ok(found);
  state = engine.tryMove(state, JSON.stringify(found), seat);
  assert.equal(view().cities.length, 2);
  assert.equal(view().settlers.length, 0);
  const academy = view()
    .cityActions.find((c) => c.position === city)!
    .buildings.find((b) => b.name === 'Academy')!;
  assert.equal(academy.reason, null);
  state = engine.tryMove(state, JSON.stringify(academy.choices[0].action), seat);
  assert.equal(view().cities.find((c) => c.position === city)!.size, 2);
  assert.equal(JSON.parse(state).actions_left, 0);
  assert.ok(view().cityActions.every((c) => c.buildings.every((b) => b.choices.length === 0)));
  assert.ok(view().settlers.every((u) => u.destinations.length === 0));
});

test('recruitment validates the whole selection, happiness uses mood, and temple bonuses can be chosen', async () => {
  const setup = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(setup));
  const p = setup.players[seat];
  p.resources = { food: 2, wood: 4, ore: 4, mood_tokens: 4 };
  p.advances.push('Myths');
  const city = p.cities[0].position;
  const expansion: View = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(setup), seat), seat));
  p.cities.push({ ...p.cities[0], position: expansion.settlers[0].destinations[0].position });
  let state = JSON.stringify(setup);
  let visible = engine.stripSecret(state, seat);
  const recruit = JSON.parse(engine.webRecruitPreview(visible, seat, city, JSON.stringify({ infantry: 2 })));
  assert.deepEqual(recruit.payment, { food: 2, ore: 2 });
  assert.throws(() => engine.webRecruitPreview(visible, seat, city, JSON.stringify({ settlers: 3 })));
  assert.throws(() => engine.webRecruitPreview(visible, 1 - seat, city, JSON.stringify({ infantry: 1 })));
  assert.throws(() => engine.webRecruitPreview(visible, seat, 'Z99', JSON.stringify({ infantry: 1 })));
  const recruited = JSON.parse(engine.tryMove(state, JSON.stringify(recruit.action), seat));
  assert.equal(recruited.players[seat].units.filter((u: any) => u.unit_type === 'Infantry').length, 2);
  assert.equal(recruited.actions_left, 2);
  p.cities[0].mood_state = 'Neutral';
  state = JSON.stringify(setup);
  let view: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const happy = view.cityActions[0].happiness[0];
  assert.deepEqual(happy.payment, { mood_tokens: 1 });
  state = engine.tryMove(state, JSON.stringify(happy.action), seat);
  assert.equal(JSON.parse(state).players[seat].cities[0].mood_state, 'Happy');
  view = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const temple = view.cityActions[0].buildings.find((b) => b.name === 'Temple')!;
  state = engine.tryMove(state, JSON.stringify(temple.choices[0].action), seat);
  view = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  assert.equal(view.choiceDecision?.choices.length, 2);
  assert.equal(
    JSON.parse(engine.webView(engine.stripSecret(state, 1 - seat), 1 - seat)).choiceDecision,
    null,
  );
  const bonus = view.choiceDecision!.choices.find((c) => c.pile?.culture_tokens)!;
  state = engine.tryMove(state, JSON.stringify(bonus.action), seat);
  assert.equal(JSON.parse(state).players[seat].resources.culture_tokens, 1);
  assert.equal(JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)).choiceDecision, null);
});

test('offset hex coordinates preserve the odd-column layout', () => {
  assert.deepEqual(positionXY('A1'), [0, 0]);
  assert.deepEqual(positionXY('B1'), [1.5, Math.sqrt(3) / 2]);
  assert.equal(positionXY('D7')[1] - positionXY('D2')[1], 5 * Math.sqrt(3));
});

test('Engineering exposes the drawn wonder only to its owner and scores have an exact breakdown for 2–4 players', async () => {
  for (const count of [2, 3, 4]) {
    const state = await engine.init(
      count,
      [],
      { undo: 'SamePlayer', civilization: 'Random' },
      `score-${count}`,
      {},
    );
    const view: View = JSON.parse(engine.webView(engine.stripSecret(state, undefined), undefined));
    assert.equal(view.players.length, count);
    for (const player of view.players) {
      assert.equal(player.scoreParts.length, 6);
      assert.equal(
        player.scoreParts.reduce((sum, part) => sum + part.points, 0),
        player.score,
      );
    }
    assert.deepEqual(view.wonderCards, []);
  }
  let state = await initial();
  const seat = engine.currentPlayer(state);
  const before: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const engineering = before.advances.find((a) => a.id === 'Engineering')!;
  assert.ok(engineering.action);
  state = engine.tryMove(state, JSON.stringify(engineering.action), seat);
  const after: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  assert.equal(after.wonderCards.length, 1);
  const card = after.wonderCards[0];
  assert.equal(card.id, JSON.parse(state).players[seat].wonder_cards[0]);
  assert.ok(card.name && card.description && card.requiredAdvance);
  assert.ok(Object.values(card.cost).some((n) => n! > 0));
  assert.ok(card.builtPoints > 0 && card.ownedPoints > 0);
  assert.equal(
    after.players[seat].scoreParts.find((part) => part.name === 'Wonders')!.points,
    0,
    'A wonder in hand scores no points',
  );
  assert.ok(
    after.players[seat].scoreParts.find((part) => part.name === 'Advances')!.points >
      before.players[seat].scoreParts.find((part) => part.name === 'Advances')!.points,
  );
  assert.equal(
    after.players[seat].scoreParts.reduce((sum, part) => sum + part.points, 0),
    after.players[seat].score,
  );
  for (const index of [undefined, 1 - seat]) {
    const stripped = engine.stripSecret(state, index);
    assert.deepEqual(JSON.parse(engine.webView(stripped, index)).wonderCards, []);
    assert.deepEqual(
      JSON.parse(engine.webView(stripped, seat)).wonderCards,
      [],
      'Cannot resolve a Hidden wonder by selecting another seat',
    );
  }
});

test('activation previews match actual mood changes, angry blocking and turn reset', async () => {
  let state = await initial();
  const seat = engine.currentPlayer(state);
  const view = () => JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)) as View;
  const collect = () => {
    const city = view().cities[0];
    const preview = JSON.parse(
      engine.webCollectPreview(
        engine.stripSecret(state, seat),
        seat,
        city.position,
        JSON.stringify([{ ...city.choices[0], times: 1 }]),
      ),
    );
    state = engine.tryMove(state, JSON.stringify(preview.action), seat);
  };
  assert.equal(view().cities[0].activations, 0);
  assert.equal(view().cities[0].activationMood, 'Happy');
  collect();
  assert.equal(view().cities[0].mood, 'Happy');
  assert.equal(view().cities[0].activationMood, 'Neutral');
  const capacity = view().cities[0].activationCapacity;
  collect();
  assert.equal(view().cities[0].mood, 'Neutral');
  assert.equal(view().cities[0].capacity, capacity);
  assert.equal(view().cities[0].activationMood, 'Angry');
  collect();
  assert.equal(view().cities[0].mood, 'Angry');
  assert.equal(view().cities[0].activations, 3);
  state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), seat);
  state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), 1 - seat);
  assert.equal(view().cities[0].activations, 0);
  collect();
  assert.equal(view().cities[0].canActivate, false);
  assert.throws(collect, 'An angry city cannot activate twice');
});
