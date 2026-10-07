import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { createViewer } from '@boardgamers/protocol/viewer';
import { ChatController } from '@boardgamers/protocol/chat';
import { journal, positionXY } from './model.ts';
import type { View, Game } from './types.ts';
const require = createRequire(import.meta.url);
const engine = require('../.engine/server.js');
async function initial() {
  return engine.init(2, [], { civilization: 'Random' }, 'clash-preview-20260927', {});
}

test('finishing a game retains the journal winner without duplicating the BGS chat announcement', () => {
  const raw = readFileSync(
    new URL('../../server/tests/test_games/status_phase/end_game.json', import.meta.url),
    'utf8',
  );
  const clean = engine.messages(raw).data;
  const finished = engine.tryMove(clean, JSON.stringify({ Playing: 'EndTurn' }), 0);
  assert.equal(engine.ended(finished), true);
  assert.deepEqual(engine.messages(finished).messages, []);
  assert.match(JSON.stringify(JSON.parse(finished).log), /wins the game/);
});

test('published WASM opens a frozen 0.4.16 save and preserves its journal for all viewers', () => {
  const raw = readFileSync(
    new URL('../../server/tests/fixtures/legacy_0_4_16/setup.json', import.meta.url),
    'utf8',
  );
  const original = JSON.parse(raw);
  assert.equal(engine.currentPlayer(raw), original.current_player_index);
  const oldJournal = original.log[0].rounds[0].turns[0].actions[0].log;
  for (const seat of [undefined, 0, 1]) {
    const visible = engine.stripSecret(raw, seat);
    const game = JSON.parse(visible);
    assert.deepEqual(game.log[0].rounds[0].turns[0].actions[0].log, oldJournal);
    assert.ok(engine.logLength(raw) > 0);
    const view: View = JSON.parse(engine.webView(visible, seat));
    assert.ok(view);
    assert.deepEqual(game.players.map((p: any) => p.cities), original.players.map((p: any) => p.cities));
  }
});

test('sea route guide follows Navigation perimeter rules and stops at unexplored terrain without revealing it', () => {
  const raw = readFileSync(
    new URL('../../server/tests/test_games/movement/ship_navigation_unit_test.json', import.meta.url),
    'utf8',
  );
  const visible = engine.stripSecret(raw, 1);
  const view: View = JSON.parse(engine.webView(visible, 1));
  for (const [from, to] of [
    ['B3', 'B5'],
    ['B5', 'A7'],
    ['A7', 'F7'],
    ['G7', 'G3'],
    ['G3', 'B3'],
  ]) {
    assert.ok(
      view.seaRoutes?.some((path) => path[0] === from && path.at(-1) === to),
      `${from} → ${to}`,
    );
    assert.ok(
      view.seaRoutes?.some((path) => path[0] === to && path.at(-1) === from),
      `${to} → ${from}`,
    );
  }
  const hidden = JSON.parse(visible);
  const example = view.seaRoutes!.find((path) => path.length > 2)!;
  const fog = example[1];
  hidden.map.tiles = hidden.map.tiles.map(([p, terrain]: [string, string]) => [
    p,
    p === fog ? 'Unexplored' : terrain,
  ]);
  const partial: View = JSON.parse(engine.webView(JSON.stringify(hidden), 1));
  assert.ok(partial.seaRoutes?.some((path) => path[0] === example[0] && path.at(-1) === fog));
  const terrain = new Map<string, string>(hidden.map.tiles);
  for (const path of partial.seaRoutes ?? []) {
    assert.equal(terrain.get(path[0]), 'Water');
    assert.ok(['Water', 'Unexplored'].includes(terrain.get(path.at(-1)!)!));
    assert.ok(path.slice(1, -1).every((p) => !['Water', 'Unexplored'].includes(terrain.get(p)!)));
  }
  const spectator: View = JSON.parse(engine.webView(JSON.stringify(hidden), undefined));
  assert.deepEqual(spectator.seaRoutes, partial.seaRoutes);
});

test('settlers can explore from filtered state and a placement choice completes both legal orientations', () => {
  const state = readFileSync(
    new URL('../../server/tests/test_games/movement/explore_resolution.json', import.meta.url),
    'utf8',
  );
  const seat = engine.currentPlayer(state);
  const visible = engine.stripSecret(state, seat);
  const before: View = JSON.parse(engine.webView(visible, seat));
  assert.equal(before.explorationDecision, null);
  const destination = before.settlers[0].destinations.find((d) => d.position === 'D6')!;
  assert.equal(destination.terrain, 'Unexplored');
  // Querying a destination never exposes its still-hidden terrain.
  assert.ok(
    JSON.parse(visible).map.unexplored_blocks.every((b: any) =>
      b.block.terrain.every((t: string) => t === 'Unexplored'),
    ),
  );
  const revealed = engine.tryMove(state, JSON.stringify(destination.action), seat);
  const decisionView: View = JSON.parse(engine.webView(engine.stripSecret(revealed, seat), seat));
  assert.equal(decisionView.supportedPhase, true);
  assert.equal(decisionView.canUndo, false);
  assert.equal(decisionView.canEndTurn, false);
  assert.ok(decisionView.settlers.every((u) => u.destinations.length === 0));
  const decision = decisionView.explorationDecision!;
  assert.equal(decision.destination, 'D6');
  assert.deepEqual(
    decision.choices.map((c) => c.rotation),
    [0, 3],
  );
  for (const index of [undefined, 99, 1 - seat]) {
    const other: View = JSON.parse(engine.webView(engine.stripSecret(revealed, index), index));
    assert.equal(other.explorationDecision ?? null, null);
  }
  for (const choice of decision.choices) {
    assert.equal(choice.tiles.length, 4);
    assert.ok(choice.tiles.every(([, terrain]) => terrain !== 'Unexplored'));
    const resolved = engine.tryMove(revealed, JSON.stringify(choice.action), seat);
    const game: Game = JSON.parse(resolved);
    for (const [position, terrain] of choice.tiles)
      assert.deepEqual(game.map.tiles.find(([p]) => p === position)?.[1], terrain);
    assert.equal(game.players[seat].units?.find((u) => u.id === 0)?.position, 'D6');
    const after: View = JSON.parse(engine.webView(engine.stripSecret(resolved, seat), seat));
    assert.equal(after.explorationDecision, null);
    assert.equal(after.supportedPhase, true);
    assert.ok(after.settlers[0].foundAction);
  }
});

test('forced exploration placement resolves automatically without moving settlers onto water', () => {
  const state = readFileSync(
    new URL('../../server/tests/test_games/movement/explore_auto_no_walk_on_water.json', import.meta.url),
    'utf8',
  );
  const seat = engine.currentPlayer(state);
  const before: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const destination = before.settlers[0].destinations.find((d) => d.position === 'B2')!;
  assert.equal(destination.terrain, 'Unexplored');
  const resolved = engine.tryMove(state, JSON.stringify(destination.action), seat);
  const game: Game = JSON.parse(resolved);
  assert.equal(game.players[seat].units?.find((u) => u.id === 0)?.position, 'B2');
  assert.notEqual(game.map.tiles.find(([p]) => p === 'B2')?.[1], 'Water');
  const after: View = JSON.parse(engine.webView(engine.stripSecret(resolved, seat), seat));
  assert.equal(after.explorationDecision, null);
  assert.equal(after.supportedPhase, true);
});

test('exploring starts one movement action and waiting players receive no exploration moves', async () => {
  let state = await initial();
  const seat = engine.currentPlayer(state);
  const actions = JSON.parse(state).actions_left;
  const before: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const destination = before.settlers.flatMap((u) => u.destinations).find((d) => d.terrain === 'Unexplored')!;
  assert.ok(destination);
  const waiting: View = JSON.parse(engine.webView(engine.stripSecret(state, 1 - seat), 1 - seat));
  assert.ok(waiting.settlers.every((u) => u.destinations.length === 0));
  state = engine.tryMove(state, JSON.stringify(destination.action), seat);
  assert.equal(JSON.parse(state).actions_left, actions - 1);
  const decision = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)).explorationDecision;
  if (decision) state = engine.tryMove(state, JSON.stringify(decision.choices[0].action), seat);
  assert.equal(JSON.parse(state).actions_left, actions - 1);
});

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
  const publicHistory = JSON.parse(engine.stripSecret(claimed, 1 - seat)).board_history;
  assert.ok(
    publicHistory.frames
      .at(-1)
      .effects.some((e: any) => e.kind === 'completed' && e.label === 'Legacy' && e.player === seat),
  );
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
  assert.ok(
    JSON.parse(engine.stripSecret(kept, 1 - seat)).board_history.frames.every(
      (f: any) => !f.title.includes('Legacy'),
    ),
    'Skipping a private objective never names it in public replay',
  );
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
    const state = await engine.init(count, [], { civilization: 'Random' }, `score-${count}`, {});
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

test('research displays a stable flexible cost and offers executable food, idea, gold and mixed payments', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  for (const resources of [{ food: 2 }, { ideas: 2 }, { gold: 2 }, { food: 1, ideas: 1 }, {}]) {
    raw.players[seat].resources = resources;
    const state = JSON.stringify(raw);
    const view: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
    const research = view.advances.find((a) => a.id === 'Storage')!;
    assert.equal(research.costAmount, 2);
    assert.deepEqual(research.costResources, ['food', 'ideas', 'gold']);
    if (!Object.keys(resources).length) {
      assert.equal(research.action, null);
      assert.deepEqual(research.payments, []);
    } else {
      assert.equal(research.payments.length, 1);
      assert.deepEqual(research.payments[0].payment, resources);
      const after = JSON.parse(engine.tryMove(state, JSON.stringify(research.payments[0].action), seat));
      assert.ok(after.players[seat].advances.includes('Storage'));
      assert.equal(after.players[seat].incident_tokens, 2);
    }
  }
  raw.players[seat].resources = { food: 2, ideas: 2, gold: 2 };
  const state = JSON.stringify(raw);
  const view: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const choices = view.advances.find((a) => a.id === 'Storage')!.payments;
  assert.equal(choices.length, 6);
  for (const choice of choices) {
    const after = JSON.parse(engine.tryMove(state, JSON.stringify(choice.action), seat));
    for (const resource of ['food', 'ideas', 'gold'] as const)
      assert.equal(after.players[seat].resources[resource] ?? 0, 2 - (choice.payment[resource] ?? 0));
  }
  const waiting: View = JSON.parse(engine.webView(engine.stripSecret(state, 1 - seat), 1 - seat));
  assert.ok(waiting.advances.every((a) => a.payments.every((p) => p.action === null)));
  assert.ok(view.advances.find((a) => a.id === 'Farming')!.payments.every((p) => p.action === null));
});

test('research discounts produce a free payment while still consuming an action and event marker', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  raw.players[seat].advances.push('Math', 'Engineering');
  raw.players[seat].resources = {};
  const state = JSON.stringify(raw);
  const view: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
  const roads = view.advances.find((a) => a.id === 'Roads')!;
  assert.equal(roads.costAmount, 0);
  assert.deepEqual(roads.costResources, []);
  assert.deepEqual(
    roads.payments.map((p) => p.payment),
    [{}],
  );
  const after = JSON.parse(engine.tryMove(state, JSON.stringify(roads.payments[0].action), seat));
  assert.equal(after.actions_left, raw.actions_left - 1);
  assert.equal(after.players[seat].incident_tokens, 2);
});

test('public civilization inspection includes researched and civilization advances and event markers for opponents and spectators', async () => {
  const raw = JSON.parse(await initial());
  raw.players[0].advances.push('Fishing', 'Engineering');
  raw.players[0].civilization = 'Vikings';
  raw.players[0].special_advances = ['ShipConstruction'];
  raw.players[0].incident_tokens = 1;
  raw.players[1].great_library_advance = 'Writing';
  const state = JSON.stringify(raw);
  for (const seat of [0, 1, undefined]) {
    const view: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
    assert.equal(view.players[0].eventTokens, 1);
    assert.deepEqual(
      view.players[0].advances.map((a) => a.id).sort(),
      ['Farming', 'Mining', 'Fishing', 'Engineering', 'ShipConstruction'].sort(),
    );
    assert.ok(view.players[0].advances.every((a) => a.name && a.description && a.group));
    assert.equal(view.players[1].advances.find((a) => a.id === 'Writing')?.borrowed, true);
    assert.equal(view.players[0].advances.find((a) => a.id === 'Engineering')?.borrowed, false);
    assert.ok(view.players.every((p) => p.advances.every((a) => !('action' in a) && !('payment' in a))));
  }
});

test('all civilizations expose four public automatic advances, exact prerequisites and structured leader abilities', async () => {
  let state = await engine.init(4, [], { civilization: 'ChooseCivilization' }, 'civilization-preview', {});
  const expected: Record<string, Record<string, string[]>> = {
    Rome: {
      Aqueduct: ['Engineering'],
      RomanRoads: ['Roads'],
      Captivi: ['Bartering'],
      Provinces: ['Dogma', 'Nationalism', 'Voting'],
    },
    Greece: {
      Study: ['PublicEducation'],
      Sparta: ['Draft'],
      HellenisticCulture: ['Arts'],
      CityStates: ['Dogma', 'Nationalism', 'Voting'],
    },
    China: {
      RiceCultivation: ['Irrigation'],
      Expansion: ['Husbandry'],
      Fireworks: ['Metallurgy'],
      ImperialArmy: ['Dogma', 'Nationalism', 'Voting'],
    },
    Vikings: {
      ShipConstruction: ['Fishing'],
      Longships: ['WarShips'],
      Raiding: ['TradeRoutes'],
      RuneStones: ['Rituals'],
    },
  };
  for (const name of Object.keys(expected)) {
    state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: name }), engine.currentPlayer(state));
  }
  const spectator: View = JSON.parse(engine.webView(engine.stripSecret(state, undefined), undefined));
  assert.deepEqual(spectator.players.map((p) => p.civilization).sort(), Object.keys(expected).sort());
  for (const player of spectator.players) {
    assert.equal(player.civilizationAdvances.length, 4);
    for (const advance of player.civilizationAdvances) {
      assert.ok(advance.name && advance.description && advance.requirement);
      assert.equal(advance.owned, false);
      assert.equal(advance.active, false);
      assert.deepEqual(
        advance.prerequisites.map((p) => p.id).sort(),
        expected[player.civilization][advance.id],
      );
      assert.ok(!('action' in advance) && !('payment' in advance));
    }
    const owner: View = JSON.parse(engine.webView(engine.stripSecret(state, player.index), player.index));
    assert.deepEqual(
      owner.players.map((p) => p.civilizationAdvances),
      spectator.players.map((p) => p.civilizationAdvances),
    );
    for (const special of player.civilizationAdvances)
      for (const required of special.prerequisites)
        assert.equal(required.name, owner.advances.find((a) => a.id === required.id)?.name);
    const leaders = owner.cityActions[0].leaders!;
    assert.equal(leaders.length, 3);
    for (const leader of leaders) {
      assert.equal(leader.abilities.length, 2);
      assert.ok(leader.abilities.every((a) => a.name && a.description));
      assert.equal(leader.description, leader.abilities.map((a) => `${a.name}: ${a.description}`).join('\n'));
    }
  }
});

test('automatic advances unlock with their research, with no extra action or event marker, and update for spectators', async () => {
  for (const [civilization, research, special, prerequisites] of [
    ['Vikings', 'Fishing', 'ShipConstruction', []],
    ['Rome', 'Engineering', 'Aqueduct', []],
    ['Greece', 'PublicEducation', 'Study', ['Writing']],
    ['China', 'Irrigation', 'RiceCultivation', []],
    ['Rome', 'Voting', 'Provinces', ['Writing', 'Philosophy']],
  ] as const) {
    const raw = JSON.parse(await initial());
    const seat = engine.currentPlayer(JSON.stringify(raw));
    raw.players[seat].civilization = civilization;
    raw.players[seat].special_advances = [];
    raw.players[seat].advances.push(...prerequisites);
    const state = JSON.stringify(raw);
    const before: View = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
    assert.equal(before.players[seat].civilizationAdvances.find((a) => a.id === special)!.active, false);
    const action = before.advances.find((a) => a.id === research)!.action;
    assert.ok(action, `${civilization}: ${research} is available`);
    const afterState = engine.tryMove(state, JSON.stringify(action), seat);
    const after: View = JSON.parse(engine.webView(engine.stripSecret(afterState, undefined), undefined));
    assert.equal(after.players[seat].civilizationAdvances.find((a) => a.id === special)!.active, true);
    assert.equal(after.players[seat].civilizationAdvances.filter((a) => a.owned).length, 1);
    assert.equal(after.players[seat].score, before.players[seat].score + 1);
    assert.equal(after.players[seat].eventTokens, before.players[seat].eventTokens - 1);
    assert.equal(JSON.parse(afterState).actions_left, raw.actions_left - 1);
  }
});

test('borrowing a prerequisite from the Great Library does not unlock a civilization advance', async () => {
  const raw = JSON.parse(await initial());
  raw.players[0].civilization = 'Vikings';
  raw.players[0].great_library_advance = 'Fishing';
  const state = JSON.stringify(raw);
  const view: View = JSON.parse(engine.webView(engine.stripSecret(state, undefined), undefined));
  assert.equal(view.players[0].civilization, 'Vikings');
  assert.equal(view.players[0].advances.find((a) => a.id === 'Fishing')?.borrowed, true);
  assert.equal(view.players[0].civilizationAdvances.find((a) => a.id === 'ShipConstruction')!.owned, false);
});

test('objective progress uses current resources, research and changing opponents, without exposing hands', async () => {
  const game = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(game)),
    other = 1 - seat;
  const p = game.players[seat];
  p.objective_cards = [6, 10, 11, 12];
  p.resources = { ...p.resources, food: 3, ore: 1, wood: 7, gold: 0 };
  const objectives = () =>
    (
      JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat)) as View
    ).objectiveCards.flatMap((c) => c.objectives);
  const get = (name: string) => objectives().find((o) => o.name === name)!;
  const storage = get('Optimized Storage');
  assert.deepEqual(
    storage.progress?.map((p) => [p.label, p.current, p.target]),
    [
      ['Food', 3, 3],
      ['Ore', 1, 3],
      ['Wood', 7, 3],
    ],
  );
  assert.equal(storage.conditionMet, false);
  p.resources.ore = 3;
  assert.equal(get('Optimized Storage').conditionMet, true);
  p.resources.food = 2;
  assert.equal(get('Optimized Storage').conditionMet, false, 'Losing resources removes eligibility');
  const planner = get('City Planner');
  assert.equal(planner.progress?.[0].target, 4);
  p.advances.push('Engineering');
  assert.equal(get('City Planner').progress?.[0].current, (planner.progress?.[0].current ?? 0) + 1);
  const ships = (count: number) =>
    Array.from({ length: count }, (_, id) => ({ id, position: 'C2', unit_type: 'Ship' }));
  p.units = ships(2);
  game.players[other].units = ships(1);
  assert.equal(get('Large Fleet').progress?.[0].target, 2);
  assert.equal(get('Large Fleet').conditionMet, true);
  game.players[other].units = ships(2);
  assert.equal(get('Large Fleet').progress?.[0].target, 3);
  assert.equal(get('Large Fleet').conditionMet, false, 'A tie does not lead');
  game.players[other].units = ships(6);
  p.units = ships(4);
  assert.equal(get('Large Fleet').progress?.[0].target, 4);
  assert.equal(get('Large Fleet').conditionMet, true, 'Four ships satisfy the alternative condition');
  assert.equal(get('Large Fleet').timing, 'Status phase');
  assert.equal(get('Large Fleet').scoringAge, game.age);
  const privateState = engine.stripSecret(JSON.stringify(game), seat);
  assert.deepEqual(JSON.parse(engine.webView(privateState, other)).objectiveCards, []);
  assert.deepEqual(JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game)))).objectiveCards, []);
});
