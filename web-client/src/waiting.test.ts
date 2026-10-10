import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const view = (state: string, seat?: number): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));

async function capture() {
  const game = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/tactics_cards/heavy_resistance.json', import.meta.url),
      'utf8',
    ),
  );
  game.players.push(...JSON.parse(await engine.init(2, [], {}, 'waiting', {})).players.slice(2));
  game.round = 1;
  game.actions_left = 0;
  game.state = { Movement: { movement_actions_left: 2 } };
  game.players.forEach((p: any) => {
    p.action_cards = [];
    p.objective_cards = [];
  });
  game.players[0].units = game.players[0].units.filter((u: any) => [0, 2].includes(u.id));
  game.players[1].units = game.players[1].units.slice(0, 1);
  // Fanaticism only replaces infantry after losing a battle at a Temple city.
  game.players[1].cities.find((city: any) => city.position === 'C1').city_pieces = { temple: 1 };
  game.players[1].cities.push(
    { position: 'B1', mood_state: 'Neutral' },
    { position: 'D2', mood_state: 'Neutral' },
  );
  game.players[0].objective_cards = [2, 8]; // Both can complete Conqueror; the choice stays private.
  game.dice_roll_outcomes = [0, 11, 11];
  return engine.tryMove(
    JSON.stringify(game),
    JSON.stringify({ Movement: { Move: { units: [0, 2], destination: 'C1', payment: {} } } }),
    0,
  ) as string;
}
function chooseFirst(state: string): string {
  const player = engine.currentPlayer(state);
  const visible = view(state, player);
  if (visible.objectiveDecision)
    return engine.tryMove(state, JSON.stringify(visible.objectiveDecision.cards[0].action), player);
  const decision = visible.decision!;
  const { action } = JSON.parse(
    engine.webQuery(
      engine.stripSecret(state, player),
      player,
      JSON.stringify({ kind: 'decision', values: [decision.options[0].value], payments: [] }),
    ),
  );
  return engine.tryMove(state, JSON.stringify(action), player);
}
function waiting(state: string, expected: NonNullable<View['waitingFor']>) {
  for (const seat of [0, 1, undefined]) assert.deepEqual(view(state, seat).waitingFor, expected);
}

test('waiting follows the defender through Fanaticism, Settler, objective choice, and back to End turn', async () => {
  let state = await capture();
  waiting(state, { player: 1, action: 'Place an infantry', source: 'Fanaticism' });
  assert.ok(!view(state, 0).decision);
  state = chooseFirst(state);
  waiting(state, { player: 1, action: 'Place a settler after losing a city', source: null });
  state = chooseFirst(state);
  waiting(state, { player: 0, action: 'Choose an objective to complete', source: null });
  assert.deepEqual(view(state, 1).players[0].completedObjectives, []);
  assert.ok(!JSON.stringify(view(state, 1).waitingFor).includes('Conqueror'));
  state = chooseFirst(state);
  waiting(state, { player: 0, action: 'End their turn', source: null });
  for (const seat of [0, 1, undefined]) {
    const completed = view(state, seat).players[0].completedObjectives!;
    assert.equal(completed.length, 1);
    assert.equal(completed[0].name, 'Conqueror');
    assert.ok(completed[0].description.length > 10);
    assert.equal(completed[0].points, 2);
    assert.ok(!JSON.stringify(completed).includes('Science Lead'));
  }
  state = engine.tryMove(state, JSON.stringify({ Playing: 'EndTurn' }), 0);
  assert.equal(view(state, 0).waitingFor, null);
});

test('movement waiting and finished games have no stale End turn indication', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'waiting-end', {}));
  game.state = { Movement: { movement_actions_left: 2 } };
  const player = game.current_player_index;
  waiting(JSON.stringify(game), { player, action: 'Finish moving', source: null });
  game.state = 'Finished';
  game.actions_left = 0;
  assert.equal(view(JSON.stringify(game)).waitingFor, null);
});

test('card decisions do not reveal another player’s hand in the waiting label', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'waiting-seer', {}));
  const player = engine.currentPlayer(JSON.stringify(game));
  game.players[player].action_cards = [158];
  const state = engine.tryMove(
    JSON.stringify(game),
    JSON.stringify({ Playing: { ActionCard: 158 } }),
    player,
  );
  waiting(state, { player, action: 'Choose a card', source: null });
});
