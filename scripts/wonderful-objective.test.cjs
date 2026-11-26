// Real WASM engine tests for Wonderful (O32, internal "Magnificent Culture").
// Build with: bash web-client/scripts/build-bridge.sh
const assert = require('node:assert/strict');
const { test } = require('node:test');
const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const move = (game, action, player = 0) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), player));

const offered = (g) =>
  (g.events || []).some((e) => e.event_type?.SelectObjectives?.shown_objective === 'Magnificent Culture');

const statusObjectiveOffered = (g, player) =>
  (g.events || []).some(
    (e) =>
      e.player === player &&
      e.event_type?.SelectObjectives?.shown_objective === 'Magnificent Culture',
  );

function wonderLogAction(player, wonder = 'Pyramids') {
  return {
    action: { Playing: 'EndTurn' },
    player,
    active_events: 0,
    items: [
      {
        player,
        HandCard: {
          card: { Wonder: wonder },
          from: { Hand: player },
          to: 'PlayToKeep',
        },
        origin: { Ability: 'Build Wonder' },
        modifiers: [],
      },
    ],
    log: [],
    undo: [],
  };
}

async function baseGame(seed) {
  const game = JSON.parse(await engine.init(2, [], {}, seed, null));
  game.current_player_index = 0;
  game.starting_player_index = 0;
  game.players[0].objective_cards = [32];
  game.players[1].objective_cards = [32];
  game.players[0].advances = ['Farming', 'Mining', 'Engineering', 'Myths', 'Rituals', 'Monuments', 'Arts', 'Sports'];
  game.players[0].resources = { food: 10, wood: 10, ore: 10, ideas: 10, gold: 10, mood_tokens: 10, culture_tokens: 10 };
  game.players[1].resources = { food: 10, wood: 10, ore: 10, ideas: 10, gold: 10, mood_tokens: 10, culture_tokens: 10 };
  return game;
}

async function buildWonder(game, wonder = 'Pyramids') {
  game.players[0].wonder_cards = [wonder];
  // A size-1 city needs a second owned city to construct a Wonder.
  const secondPos = game.map.tiles.find(
    ([pos, terrain]) =>
      terrain !== 'Water' &&
      !game.players.some((p) => (p.cities || []).some((c) => c.position === pos)),
  )[0];
  game.players[0].cities.push({ mood_state: 'Neutral', position: secondPos });
  game.players[0].cities[0].mood_state = 'Happy';
  let next = move(game, { Playing: { WonderCard: wonder } }, 0);
  const view = JSON.parse(engine.webView(JSON.stringify(next), 0));
  const payment = view.decision?.fields?.[0]?.choices?.[0] ?? view.decision?.fields?.[0]?.initial;
  assert.ok(payment, 'expected Wonder payment choice');
  return move(next, { Response: { Payment: [payment] } }, 0);
}

test('Immediate: sole Wonder builder qualifies on 1st and 2nd Wonder, but not after opponent builds', async () => {
  // 1st Wonder by player 0 -> offered.
  const g1 = await buildWonder(await baseGame('wonderful-imm-1'), 'Pyramids');
  assert.ok(offered(g1), '1st Wonder should offer Wonderful immediately');

  // 2nd Wonder by player 0 (already built 1 earlier) -> still offered under corrected rules.
  const g2Base = await baseGame('wonderful-imm-2');
  const g2Turn = g2Base.log.at(-1).rounds.at(-1).turns.at(-1);
  g2Turn.actions = [...(g2Turn.actions || []), wonderLogAction(0, 'Colosseum')];
  g2Base.log_index = g2Turn.actions.length;
  const g2 = await buildWonder(g2Base, 'Pyramids');
  assert.ok(offered(g2), '2nd Wonder by sole builder should offer Wonderful immediately');

  // Opponent built earlier -> NOT offered.
  const g3Base = await baseGame('wonderful-imm-3');
  const g3Turn = g3Base.log.at(-1).rounds.at(-1).turns.at(-1);
  g3Turn.actions = [...(g3Turn.actions || []), wonderLogAction(1, 'Colosseum')];
  g3Base.log_index = g3Turn.actions.length;
  const g3 = await buildWonder(g3Base, 'Pyramids');
  assert.ok(!offered(g3), 'should not offer Wonderful once another player built a Wonder');
});

async function runStatusPhase(seed, configureLog) {
  const game = await baseGame(seed);
  // Position at the final turn of Round 3 so EndTurn enters Status Phase directly.
  game.round = 3;
  game.current_player_index = 1;
  game.actions_left = 0;
  configureLog(game);
  return move(game, { Playing: 'EndTurn' }, 1);
}

test('Status Phase: cross-round same-Age Wonder by opponent disqualifies both players', async () => {
  const after = await runStatusPhase('wonderful-status-cross', (game) => {
    game.log = [
      {
        age: 1,
        rounds: [
          { round: 1, turns: [{ turn_type: { Player: 1 }, actions: [wonderLogAction(1, 'Colosseum')] }] },
          { round: 3, turns: [{ turn_type: { Player: 0 }, actions: [wonderLogAction(0, 'Pyramids')] }, { turn_type: { Player: 1 }, actions: [] }] },
        ],
      },
    ];
    game.log_index = 0;
  });
  assert.ok(!statusObjectiveOffered(after, 0), 'P0 must not qualify when P1 built in R1 of same Age');
  assert.ok(!statusObjectiveOffered(after, 1), 'P1 must not qualify when P0 built in R3 of same Age');
});

test('Status Phase: earlier-round sole Wonder builder qualifies even when no Wonder is built in R3', async () => {
  const after = await runStatusPhase('wonderful-status-earlier', (game) => {
    game.log = [
      {
        age: 1,
        rounds: [
          { round: 1, turns: [{ turn_type: { Player: 0 }, actions: [wonderLogAction(0, 'Pyramids')] }] },
          { round: 3, turns: [{ turn_type: { Player: 1 }, actions: [] }] },
        ],
      },
    ];
    game.log_index = 0;
  });
  assert.ok(statusObjectiveOffered(after, 0), 'P0 should qualify in Status Phase for R1 sole Wonder');
  const claimed = move(after, { Response: { SelectHandCards: [{ ObjectiveCard: 32 }] } }, 0);
  assert.deepEqual(claimed.players[0].completed_objectives, [{ card: 32, name: 'Magnificent Culture' }]);
});

test('Status Phase: previous-Age Wonder by opponent does not disqualify sole builder in current Age', async () => {
  const after = await runStatusPhase('wonderful-status-prev-age', (game) => {
    game.age = 2;
    game.log = [
      {
        age: 1,
        rounds: [{ round: 2, turns: [{ turn_type: { Player: 1 }, actions: [wonderLogAction(1, 'Colosseum')] }] }],
      },
      {
        age: 2,
        rounds: [
          { round: 2, turns: [{ turn_type: { Player: 0 }, actions: [wonderLogAction(0, 'Pyramids')] }] },
          { round: 3, turns: [{ turn_type: { Player: 1 }, actions: [] }] },
        ],
      },
    ];
    game.log_index = 0;
  });
  assert.ok(statusObjectiveOffered(after, 0), 'P0 should qualify in Age 2 despite P1 building in Age 1');
});
