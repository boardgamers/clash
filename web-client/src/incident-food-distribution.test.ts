import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const distributions = [
  [11, 1],
  [14, 2],
  [16, 3],
] as const;
const move = (state: string, action: unknown): string =>
  engine.tryMove(state, JSON.stringify(action), engine.currentPlayer(state));
const strip = (state: string): string => engine.stripSecret(state, 0);
const view = (state: string) => JSON.parse(engine.webView(strip(state), 0));
const food = (state: string): number[] =>
  JSON.parse(state).players.map((player: any) => player.resources?.food ?? 0);
const query = (state: string, recipient: number) =>
  JSON.parse(
    engine.webQuery(strip(state), 0, JSON.stringify({ kind: 'decision', values: [recipient], payments: [] })),
  );

// Each opponent is [food, food limit]. Keep Storage ownership consistent with the cap.
async function ready(incident: number, opponents: readonly (readonly [number, number])[]) {
  const count = opponents.length + 1;
  const game = JSON.parse(await engine.init(count, [], {}, 'incident-food-distribution', {}));
  game.current_player_index = 0;
  game.board_history = { id: 'incident-food-distribution', frames: [] };
  for (let i = 0; i < game.players.length; i++) {
    const player = game.players[i];
    player.action_cards = [];
    player.objective_cards = [];
    player.wonder_cards = [];
    if (i < count) {
      player.civilization = ['Maya', 'Rome', 'Greece', 'Egypt'][i];
      const [amount, cap] = i === 0 ? [4, 7] : opponents[i - 1];
      player.resources = { food: amount };
      player.resource_limit.food = cap;
      player.advances = ['Farming', 'Mining', ...(cap === 7 ? ['Storage'] : [])];
    } else {
      // Barbarians/Pirates have capacity, but must not be targets or block fallback.
      player.resources = {};
      player.resource_limit = { ...player.resource_limit, food: 7 };
      player.units = [];
      player.cities = [];
    }
  }
  game.players[0].advances.push('Math', 'Astronomy'); // Calendar makes recipient Undo testable.
  game.players[0].incident_tokens = 1;
  game.incidents_left = [incident, 29, 30];
  const cities = new Set(game.players.flatMap((p: any) => p.cities.map((c: any) => c.position)));
  for (const tile of game.map.tiles) {
    if (!cities.has(tile[0]) && !['Water', 'Unexplored'].includes(tile[1])) tile[1] = 'Barren';
  }
  return JSON.stringify(game);
}

function trigger(state: string): string {
  return move(state, { Playing: { Advance: { advance: 'Irrigation', payment: { food: 2 } } } });
}

function choose(state: string, recipient: number): string {
  const result = query(state, recipient);
  assert.deepEqual(result.action, { Response: { SelectPlayer: recipient } });
  return move(state, result.action);
}

function assertChoices(state: string, expected: number[]) {
  assert.equal(engine.currentPlayer(state), 0);
  assert.deepEqual(JSON.parse(state).events.at(-1).handler.request.SelectPlayer.choices, expected);
  const decision = view(state).decision;
  assert.deepEqual([decision.min, decision.max], [1, 1]);
  assert.deepEqual(
    decision.options.map((option: any) => option.value),
    expected,
  );
  assert.deepEqual(
    JSON.parse(engine.webView(state, 0)).decision.options,
    decision.options,
    'the redacted browser state must expose the same recipient choices',
  );
}

function assertFinished(state: string, incident: number, amount: number, opponents: number[]) {
  const game = JSON.parse(state);
  assert.deepEqual(game.events ?? [], [], `incident ${incident} should be resolved`);
  assert.equal(game.incidents_discarded.at(-1), incident);
  assert.deepEqual(food(state).slice(0, opponents.length + 1), [2 + amount, ...opponents]);
  assert.ok(
    food(state)
      .slice(opponents.length + 1)
      .every((amount) => amount === 0),
  );
  assert.equal(view(state).decision, null);
}

for (const [incident, amount] of distributions) {
  test(`incident ${incident}: real three/four-player choices use individual caps and permit repeat recipients`, async () => {
    for (const withFullOpponent of [false, true]) {
      const opponents: [number, number][] = withFullOpponent
        ? [
            [2, 2],
            [2, 7],
            [0, 2],
          ]
        : [
            [2, 7],
            [0, 2],
          ];
      const recipients = withFullOpponent ? [2, 3] : [1, 2];
      let state = trigger(await ready(incident, opponents));
      for (let award = 0; award < amount; award++) {
        assertChoices(state, recipients);
        state = choose(state, recipients[0]);
      }
      assertFinished(state, incident, amount, withFullOpponent ? [2, 2 + amount, 0] : [2 + amount, 0]);
    }
  });

  test(`incident ${incident}: webQuery rejects capped opponents, the actor, and NPCs`, async () => {
    const state = trigger(
      await ready(incident, [
        [2, 2],
        [0, 7],
        [0, 7],
      ]),
    );
    assertChoices(state, [2, 3]);
    for (const invalid of [0, 1, 4, 5]) {
      // The JS wrapper turns the query's validation Err into a thrown string;
      // do not invoke tryMove's Rust assertion/panic on the shared WASM instance.
      assert.throws(() => query(state, invalid), /Select one choice/);
    }
    assertChoices(state, [2, 3]);
    assert.deepEqual(query(state, 2).action, { Response: { SelectPlayer: 2 } });
  });

  test(`incident ${incident}: the sole room-bearing opponent gets all food automatically`, async () => {
    const state = trigger(
      await ready(incident, [
        [2, 2],
        [0, 7],
      ]),
    );
    assertFinished(state, incident, amount, [2, amount]);
  });

  test(`incident ${incident}: using the last capacity reopens full opponents for unavoidable waste`, async () => {
    let state = trigger(
      await ready(incident, [
        [1, 2],
        [2, 2],
      ]),
    );
    assert.equal(food(state)[1], 2, 'first food must be kept automatically');
    for (let award = 1; award < amount; award++) {
      assertChoices(state, [1, 2]);
      state = choose(state, 2);
    }
    assertFinished(state, incident, amount, [2, 2]);
  });

  test(`incident ${incident}: all-full fallback ignores actor/NPC capacity and still allows repeat recipients`, async () => {
    let state = trigger(
      await ready(incident, [
        [2, 2],
        [7, 7],
        [2, 2],
      ]),
    );
    assert.ok(food(state)[0] < 7, 'actor still has room');
    for (let award = 0; award < amount; award++) {
      assertChoices(state, [1, 2, 3]);
      state = choose(state, 1);
    }
    assertFinished(state, incident, amount, [2, 7, 2]);
  });

  test(`incident ${incident}: two-player fallback keeps what fits and automatically wastes the rest`, async () => {
    for (const initialFood of [1, 2]) {
      const state = trigger(await ready(incident, [[initialFood, 2]]));
      assertFinished(state, incident, amount, [2]);
    }
  });
}

test('food recipient choices update after each award, including automatic awards and Undo/Redo', async () => {
  const choosing = trigger(
    await ready(16, [
      [1, 2],
      [1, 2],
      [0, 7],
    ]),
  );
  assertChoices(choosing, [1, 2, 3]);
  const first = choose(choosing, 1);
  assertChoices(first, [2, 3]);
  assert.equal(food(first)[1], 2);
  assert.throws(() => query(first, 1), /Select one choice/);
  assert.equal(view(first).canUndo, true);

  const undone = move(first, 'Undo');
  assertChoices(undone, [1, 2, 3]);
  assert.equal(food(undone)[1], 1);
  const redone = move(undone, 'Redo');
  assertChoices(redone, [2, 3]);
  assert.equal(food(redone)[1], 2);

  // Player 2 fills up, so the last food goes straight to player 3.
  const resolved = choose(redone, 2);
  assertFinished(resolved, 16, 3, [2, 2, 1]);
  const undoAutomaticAward = move(resolved, 'Undo');
  assertChoices(undoAutomaticAward, [2, 3]);
  assert.deepEqual(food(undoAutomaticAward).slice(1, 4), [2, 1, 0]);
  assertFinished(move(undoAutomaticAward, 'Redo'), 16, 3, [2, 2, 1]);
});

test('fixed food rewards to all players still waste excess rather than redistribute it', async () => {
  for (const [incident, amount] of [
    [9, 1],
    [13, 2],
    [15, 3],
  ]) {
    const state = trigger(
      await ready(incident, [
        [2, 2],
        [6, 7],
        [0, 7],
      ]),
    );
    assertFinished(state, incident, amount, [2, 7, amount]);
  }
});
