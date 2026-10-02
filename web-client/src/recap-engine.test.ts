import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { passengerLandings } from './map-actions.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (name: string) =>
  readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8');
const view = (raw: string, seat: number) => JSON.parse(engine.webView(engine.stripSecret(raw, seat), seat));
const routes = (raw: string, seat: number, units: number[]) =>
  JSON.parse(
    engine.webQuery(engine.stripSecret(raw, seat), seat, JSON.stringify({ kind: 'movement', units })),
  ).destinations;

test('public board history survives reload, trims undo, and never exposes hidden cards or map blocks', async () => {
  let raw = await engine.init(2, [], { civilization: 'Random' }, 'recap-source-private-seed', {});
  const seat = engine.currentPlayer(raw),
    before = engine.logLength(raw);
  const action = view(raw, seat).advances.find((a: any) => a.id === 'Storage').action;
  raw = engine.tryMove(raw, JSON.stringify(action), seat);
  const history = JSON.parse(raw).board_history;
  assert.deepEqual(
    history.frames.map((f: any) => f.cursor),
    [before, engine.logLength(raw)],
  );
  for (const observer of [seat, 1 - seat, undefined])
    assert.deepEqual(JSON.parse(engine.stripSecret(raw, observer)).board_history, history);
  assert.doesNotMatch(
    JSON.stringify(history),
    /recap-source-private-seed|action_cards|objective_cards|wonders_left|unexplored_blocks|rng|undo/,
  );
  const undone = engine.tryMove(raw, '"Undo"', seat);
  assert.equal(engine.logLength(undone), before);
  assert.equal(JSON.parse(undone).board_history.frames.length, 1);
  const redone = engine.tryMove(undone, '"Redo"', seat);
  assert.equal(JSON.parse(redone).board_history.frames.length, 2);
  assert.deepEqual(JSON.parse(redone).board_history.frames.at(-1).players, history.frames.at(-1).players);
});

test('drawing a card records a public card-back effect, including for opponents', async () => {
  let raw = await engine.init(2, [], { civilization: 'Random' }, 'draw-recap', {}),
    seat = engine.currentPlayer(raw);
  const game = JSON.parse(raw);
  game.players[seat].resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7 };
  raw = JSON.stringify(game);
  const writing = view(raw, seat).advances.find((a: any) => a.id === 'Writing');
  assert.ok(writing.action);
  raw = engine.tryMove(raw, JSON.stringify(writing.action), seat);
  const effects = JSON.parse(engine.stripSecret(raw, 1 - seat)).board_history.frames.at(-1).effects;
  assert.ok(effects?.some((e: any) => e.player === seat && e.kind === 'action'));
  assert.ok(effects.every((e: any) => Object.keys(e).sort().join(',') === 'kind,label,player'));
});

test('analysis retains a public pending payment and resamples hidden state independently', async () => {
  const base = JSON.parse(await engine.init(2, [], { civilization: 'Random' }, 'pending-source', {}));
  base.players[0].advances.push('Writing', 'FreeEducation');
  base.players[0].advances = [...new Set(base.players[0].advances)];
  base.players[0].resources = { food: 3, ideas: 3, gold: 3, mood_tokens: 0 };
  base.current_player_index = 0;
  base.events = JSON.parse(fixture('advances/free_education.outcome')).events;
  const source = JSON.stringify(base);
  assert.equal(engine.canLaunchAnalysisMode(source), true);
  const branch = engine.createAnalysisScenario(source, { player: 0, seed: 'simulation' });
  assert.deepEqual(JSON.parse(branch).events, base.events);
  assert.equal(JSON.parse(branch).board_history, undefined);
  const other = JSON.parse(source);
  other.seed = 'different-source-seed';
  other.rng = '123';
  other.action_cards_left.reverse();
  other.objective_cards_left.reverse();
  other.wonders_left.reverse();
  assert.deepEqual(
    JSON.parse(engine.createAnalysisScenario(JSON.stringify(other), { player: 0, seed: 'simulation' })),
    JSON.parse(branch),
  );
  const decision = view(branch, 0).decision;
  assert.ok(decision);
  const response = JSON.parse(
    engine.webQuery(
      engine.stripSecret(branch, 0),
      0,
      JSON.stringify({ kind: 'decision', values: [], payments: [{ ideas: 1 }] }),
    ),
  ).action;
  const resolved = JSON.parse(engine.tryMove(branch, JSON.stringify(response), 0));
  assert.equal(resolved.players[0].resources.ideas, 2);
  assert.equal(resolved.players[0].resources.mood_tokens, 1);
  base.events[0].handler.request = {
    SelectHandCards: { choices: [], needed: { start: 0, end: 1 }, description: 'Private card' },
  };
  // Existing real private continuation shape avoids depending on hand-request serialization.
  base.events = [{ event_type: 'ChooseActionCard', player: 0 }];
  assert.equal(engine.canLaunchAnalysisMode(JSON.stringify(base)), false);
});

function seaFixture() {
  const game = JSON.parse(fixture('movement/ship_navigation_unit_test'));
  game.state = 'Playing';
  game.actions_left = 3;
  game.current_player_index = 1;
  for (const p of game.players) p.units = [];
  game.players[1].units = [{ id: 1, unit_type: 'Ship', position: 'F6' }];
  game.map.tiles = game.map.tiles.map(([p]: [string, string]) => [
    p,
    ['F5', 'F6', 'G7', 'A7', 'A3'].includes(p) ? 'Water' : 'Fertile',
  ]);
  return game;
}
test('ships can choose the far end of the sea area and the next area with Navigation', () => {
  const g = seaFixture();
  let destinations = routes(JSON.stringify(g), 1, [1]);
  for (const p of ['F5', 'G7', 'A7', 'A3'])
    assert.ok(
      destinations.some((d: any) => d.position === p),
      p,
    );
  const action = destinations.find((d: any) => d.position === 'A7').action;
  const moved = JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 1));
  assert.equal(moved.players[1].units[0].position, 'A7');
  assert.equal(moved.actions_left, 2);
  g.players[1].advances = g.players[1].advances.filter((a: string) => a !== 'Navigation');
  destinations = routes(JSON.stringify(g), 1, [1]);
  assert.ok(destinations.some((d: any) => d.position === 'G7'));
  assert.ok(!destinations.some((d: any) => d.position === 'A7'));
});
test('sea routes stop at enemy ships and unexplored regions', () => {
  const g = seaFixture();
  g.players[0].units = [{ id: 20, unit_type: 'Ship', position: 'G7' }];
  let ds = routes(JSON.stringify(g), 1, [1]);
  assert.ok(ds.some((d: any) => d.position === 'G7' && d.attack));
  assert.ok(!ds.some((d: any) => d.position === 'A7'));
  g.players[0].units = [];
  g.map.tiles = g.map.tiles.map(([p, t]: [string, string]) => [p, p === 'F7' ? 'Unexplored' : t]);
  ds = routes(JSON.stringify(g), 1, [1]);
  assert.ok(ds.some((d: any) => d.position === 'F7'));
  assert.ok(!ds.some((d: any) => d.position === 'A7'));
});
test('loaded ship offers legal passenger landings; attacking leaves its carrier at sea', () => {
  const g = seaFixture();
  g.players[1].units[0].carried_units = [
    { id: 2, unit_type: 'Infantry' },
    { id: 3, unit_type: 'Infantry' },
  ];
  g.players[1].advances.push('Tactics');
  g.players[1].advances = [...new Set(g.players[1].advances)];
  g.players[0].cities = [{ position: 'F7', mood_state: 'Neutral', city_pieces: {} }];
  const raw = JSON.stringify(g),
    v = view(raw, 1);
  const targets = passengerLandings(v, [1], (ids) => routes(raw, 1, ids));
  const landing = targets.find((d) => d.position === 'F7');
  assert.ok(landing);
  assert.deepEqual(landing.units, [2, 3]);
  const d = routes(raw, 1, landing.units).find((d: any) => d.position === 'F7');
  assert.ok(d.attack);
  const next = JSON.parse(engine.tryMove(raw, JSON.stringify(d.action), 1));
  assert.equal(next.players[1].units.find((u: any) => u.id === 1).position, 'F6');
  g.state = { Movement: { movement_actions_left: 2, moved_units: [2, 3] } } as any;
  assert.deepEqual(
    passengerLandings(view(JSON.stringify(g), 1), [1], (ids) => routes(JSON.stringify(g), 1, ids)),
    [],
  );
});

test('Cartography distinguishes crossing connected water from using Navigation', () => {
  const g = seaFixture();
  g.players[1].advances.push('Cartography');
  g.players[1].resources = {};
  // F5 to G7 crosses F6 without Navigation; A7 requires the edge passage.
  g.players[1].units[0].position = 'F5';
  for (const [position, culture] of [
    ['G7', 0],
    ['A7', 1],
  ] as const) {
    const raw = JSON.stringify(g),
      d = routes(raw, 1, [1]).find((d: any) => d.position === position);
    assert.ok(d);
    const next = JSON.parse(engine.tryMove(raw, JSON.stringify(d.action), 1));
    assert.equal(next.players[1].resources.ideas, 1);
    assert.equal(next.players[1].resources.culture_tokens ?? 0, culture);
  }
});

test('Cartography counts a Navigation exploration before its new sea area is revealed', () => {
  const g = JSON.parse(fixture('movement/ship_navigate_explore_move'));
  g.players[1].advances = [...new Set([...g.players[1].advances, 'Cartography'])];
  g.players[1].resources = {};
  const raw = JSON.stringify(g),
    d = routes(raw, 1, [1]).find((d: any) => d.position === 'F2');
  assert.ok(d);
  const next = JSON.parse(engine.tryMove(raw, JSON.stringify(d.action), 1));
  assert.equal(next.players[1].resources.ideas, 1);
  assert.equal(next.players[1].resources.culture_tokens, 1);
});
