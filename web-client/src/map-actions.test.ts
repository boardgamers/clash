import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { canMoveOnMap, moveOrigins } from './map-actions.ts';
import { movementBonus } from './movement-bonus.ts';
import { collectionYield } from './collection-yield.ts';
import { journal } from './journal.ts';
import type { Game, View, MoveDestination, Selection } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const initial = () =>
  engine.init(2, [], { undo: 'SamePlayer', civilization: 'Random' }, 'clash-preview-20260927', {});
const query = (state: string, seat: number, input: unknown) =>
  JSON.parse(engine.webQuery(engine.stripSecret(state, seat), seat, JSON.stringify(input)));
const view = (state: string, seat?: number): View =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));

test('destination-first movement offers legal units and requires no game mutation to preview', async () => {
  const state = await initial();
  const seat = engine.currentPlayer(state);
  const v = view(state, seat);
  const destinations = (units: number[]): MoveDestination[] =>
    query(state, seat, { kind: 'movement', units }).destinations;
  const targets = destinations([v.units![0].id]);
  assert.ok(targets.length);
  for (const target of targets) {
    const origins = moveOrigins(v, JSON.parse(state), target.position, destinations);
    assert.ok(origins.some((u) => u.id === v.units![0].id));
    for (const unit of origins) {
      const route = destinations([unit.id]).find((d) => d.position === target.position)!;
      assert.doesNotThrow(() => engine.tryMove(state, JSON.stringify(route.action), seat));
    }
  }
  const after = view(state, seat);
  assert.deepEqual(after.units, v.units, 'Read-only offers never move a unit');
  assert.equal(after.canUndo, v.canUndo, 'Read-only offers never perform an action');
  assert.deepEqual(after.cities, v.cities);
});

test('map movement never offers opponent, spectator, exhausted or decision-bound actions', async () => {
  const state = await initial();
  const seat = engine.currentPlayer(state);
  for (const other of [1 - seat, undefined]) {
    const v = view(state, other);
    assert.equal(canMoveOnMap(v, JSON.parse(state)), false);
    assert.deepEqual(
      moveOrigins(v, JSON.parse(state), 'D7', () => {
        throw Error('Must not query');
      }),
      [],
    );
  }
  const raw = JSON.parse(state);
  raw.actions_left = 0;
  assert.equal(canMoveOnMap(view(JSON.stringify(raw), seat), raw), false);
  const moving = readFileSync(
    new URL('../../server/tests/test_games/movement/explore_resolution.json', import.meta.url),
    'utf8',
  );
  const m = JSON.parse(moving);
  m.actions_left = 0;
  const mv = view(JSON.stringify(m), engine.currentPlayer(moving));
  assert.equal(canMoveOnMap(mv, m), true, 'An ongoing movement action can continue at zero actions');
  assert.equal(canMoveOnMap({ ...mv, choiceDecision: { name: 'Bonus', choices: [] } }, m), false);
});

test('Rice Cultivation tile yields match actual collection, cap the bonus and leave whole-action bonuses separate', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  const p = raw.players[seat];
  p.civilization = 'China';
  p.advances.push('Irrigation', 'Storage', 'Writing', 'PublicEducation');
  p.special_advances = ['RiceCultivation'];
  p.resources = {};
  p.resource_limit.food = 7;
  p.cities[0].city_pieces = { academy: seat, fortress: seat };
  raw.map.tiles = raw.map.tiles.map(([pos]: [string, string]) => [pos, 'Fertile']);
  let state = JSON.stringify(raw);
  const city = view(state, seat).cities[0];
  const tiles = city.choices.filter((c) => c.position !== city.position).slice(0, 3);
  assert.equal(tiles.length, 3);
  for (const [i, tile] of tiles.entries())
    p.units.push({ ...p.units[0], id: 10 + i, position: tile.position });
  state = JSON.stringify(raw);
  const choices = view(state, seat).cities[0].choices;
  const selected: Selection[] = tiles.map((t) => ({
    ...choices.find((c) => c.position === t.position)!,
    times: 1,
  }));
  assert.ok(selected.every((c) => c.bonuses?.[0].source === 'Rice Cultivation'));
  assert.deepEqual(
    selected.map((c) => collectionYield(c, selected)),
    [{ food: 2 }, { food: 2 }, { food: 1 }],
  );
  assert.deepEqual(collectionYield(selected[0], [selected[0]]), { food: 2 });
  assert.deepEqual(
    collectionYield(choices.find((c) => c.position === city.position)!),
    { food: 1 },
    'Settler in city gives no rice bonus',
  );
  assert.ok(
    choices.filter((c) => !tiles.some((t) => t.position === c.position)).every((c) => !c.bonuses?.length),
  );
  const result = query(state, seat, {
    kind: 'collect',
    city: city.position,
    selections: selected,
    variant: 'Collect',
  });
  assert.deepEqual(
    result.total,
    { food: 5, ideas: 1 },
    'Academy education is a once-per-action bonus, not repeated on each tile',
  );
  const after = JSON.parse(engine.tryMove(state, JSON.stringify(result.action), seat));
  assert.equal(after.players[seat].resources.food, 5);
  assert.equal(after.players[seat].resources.ideas, 1);
  assert.deepEqual(result.effects, [
    { source: 'Rice Cultivation', description: 'Added 2 food' },
    { source: 'Public Education', description: 'Gain 1 idea' },
  ]);
  const collection = journal(after)
    .reverse()
    .find((e) => e.collection);
  assert.deepEqual(
    collection?.collection?.effects.map((e) => e.source),
    ['Rice Cultivation', 'Public Education'],
    JSON.stringify(collection),
  );
  assert.deepEqual(
    selected.map((c) => c.pile),
    [{ food: 1 }, { food: 1 }, { food: 1 }],
    'Display bonuses never change submitted base choices',
  );
});

test('Expansion moves are separate from a paid Move action, and exploration preserves moved units', async () => {
  const raw = JSON.parse(await initial());
  const seat = engine.currentPlayer(JSON.stringify(raw));
  const p = raw.players[seat];
  p.civilization = 'China';
  p.advances.push('Husbandry');
  p.special_advances = ['Expansion'];
  let state = JSON.stringify(raw);
  const move = (action: unknown) => {
    state = engine.tryMove(state, JSON.stringify(action), seat);
  };
  const routes = (id: number): MoveDestination[] =>
    query(state, seat, { kind: 'movement', units: [id] }).destinations;
  move({
    Playing: {
      Recruit: { units: { settlers: 1 }, city_position: p.cities[0].position, payment: { food: 2 } },
    },
  });
  assert.equal(movementBonus(JSON.parse(state))?.source, 'Expansion');
  assert.equal(JSON.parse(state).actions_left, 2);
  move(routes(0).find((d) => d.terrain === 'Barren')!.action);
  assert.equal(movementBonus(JSON.parse(state))?.source, 'Expansion');
  assert.equal(routes(0).length, 0, 'The same settler cannot use Expansion twice');
  move(routes(1).find((d) => d.terrain === 'Forest')!.action);
  assert.equal(JSON.parse(state).state, 'Playing');
  assert.equal(JSON.parse(state).actions_left, 2, 'Expansion does not spend another action');
  assert.equal(movementBonus(JSON.parse(state)), null);

  const unexplored = routes(1).find((d) => d.terrain === 'Unexplored')!;
  assert.ok(unexplored, 'The settler may move again in a new paid action');
  move(unexplored.action);
  const exploration = view(state, seat).explorationDecision;
  if (exploration) move(exploration.choices[0].action);
  const after = JSON.parse(state);
  assert.equal(after.actions_left, 1);
  assert.deepEqual(after.state.Movement.moved_units, [1]);
  assert.equal(after.state.Movement.movement_actions_left, 2);
  assert.equal(movementBonus(after), null, 'An earlier Expansion is not attributed to a later paid move');
  assert.equal(routes(1).length, 0, 'Exploration does not reset the moved settler');
  assert.ok(routes(0).length, 'Another group can still move as part of this action');
  move(routes(0)[0].action);
  assert.equal(JSON.parse(state).actions_left, 1, 'Moving another group does not spend another action');
});
