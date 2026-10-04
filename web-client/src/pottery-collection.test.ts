import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { collectionBonusIndicators, collectionStorageWaste } from './collection-yield.ts';
import type { Selection, View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const serialize = (game: any) => JSON.stringify(game);
const view = (game: any, seat: number): View =>
  JSON.parse(engine.webView(engine.stripSecret(serialize(game), seat), seat));
const query = (game: any, seat: number, selections: Selection[]) =>
  JSON.parse(
    engine.webQuery(
      engine.stripSecret(serialize(game), seat),
      seat,
      JSON.stringify({
        kind: 'collect',
        city: game.players[seat].cities[0].position,
        selections,
        variant: 'Collect',
      }),
    ),
  );
async function setup() {
  const game = JSON.parse(await engine.init(2, [], {}, 'pottery-preview', {}));
  const seat = engine.currentPlayer(serialize(game));
  const p = game.players[seat];
  p.civilization = 'Japan';
  p.advances = ['Farming', 'Mining', 'Irrigation', 'Storage', 'Writing', 'PublicEducation'];
  p.resources = {};
  p.resource_limit.food = 7;
  p.cities[0].city_pieces = { academy: seat };
  p.cities[0].mood_state = 'Happy';
  game.map.tiles = game.map.tiles.map(([pos]: [string, unknown]) => [pos, 'Fertile']);
  return { game, seat, p };
}

test('Pottery appears at three food, combines with Education, and matches the actual once-per-turn reward', async () => {
  const { game, seat } = await setup();
  const city = view(game, seat).cities[0];
  assert.deepEqual(city.collectionBonuses, [
    { source: 'Pottery', minimum: { food: 3 }, pile: { culture_tokens: 1 } },
  ]);
  const selected = city.choices.slice(0, 3).map((c) => ({ ...c, times: 1 }));
  assert.equal(selected.length, 3);
  const before = serialize(game);
  assert.deepEqual(query(game, seat, selected.slice(0, 2)).total, { food: 2, ideas: 1 });
  const preview = query(game, seat, selected);
  assert.deepEqual(preview.total, { food: 3, ideas: 1, culture_tokens: 1 });
  assert.deepEqual(preview.effects, [
    { source: 'Public Education', description: 'Gain 1 idea' },
    { source: 'Pottery', description: 'Gain 1 culture token' },
  ]);
  assert.deepEqual(query(game, seat, selected), preview, 'Preview never consumes the bonus');
  assert.equal(serialize(game), before);
  const after = engine.tryMove(before, serialize(preview.action), seat);
  const next = JSON.parse(after);
  assert.deepEqual(next.players[seat].resources, preview.after);
  assert.equal(next.players[seat].event_info.Pottery, 'used');
  assert.deepEqual(view(next, seat).cities[0].collectionBonuses, []);
  assert(!query(next, seat, selected.slice(0, 2)).total.culture_tokens);
  const undone = JSON.parse(engine.tryMove(after, JSON.stringify('Undo'), seat));
  assert.deepEqual(query(undone, seat, selected), preview, 'Undo restores the bonus');
});

test('Pottery markers anticipate the third food and show the included bonus only once', async () => {
  const { game, seat } = await setup();
  const city = view(game, seat).cities[0];
  const selected = city.choices.slice(0, 3).map((c) => ({ ...c, times: 1 }));
  const bonus = city.collectionBonuses;
  assert.deepEqual(collectionBonusIndicators(selected[2], [], city), []);
  assert.deepEqual(collectionBonusIndicators(selected[2], selected.slice(0, 1), city), []);
  assert.deepEqual(collectionBonusIndicators(selected[2], selected.slice(0, 2), city), bonus);
  assert.deepEqual(
    selected.map((c) => collectionBonusIndicators(c, selected, city)),
    [[], [], bonus],
  );
  assert.deepEqual(collectionBonusIndicators(city.choices[3], selected, city), [], 'No bonus on extra tiles');
  assert.deepEqual(
    collectionBonusIndicators(selected[2], selected.slice(0, 2), { ...city, capacity: 2 }),
    [],
    'Do not promise an over-capacity choice',
  );
  const wood = { position: 'X1', pile: { wood: 1 } };
  assert.deepEqual(collectionBonusIndicators(wood, selected.slice(0, 2), city), []);
  assert.deepEqual(
    collectionBonusIndicators(selected[2], selected.slice(0, 2), { ...city, collectionBonuses: [] }),
    [],
  );
});

test('Focused Collection repeated food counts toward Pottery; full storage still previews the reward and waste', async () => {
  const { game, seat, p } = await setup();
  game.permanent_effects = [{ Collect: 'ProductionFocus' }];
  p.resources = { food: 7 };
  const city = view(game, seat).cities[0];
  assert.equal(city.maxPerTile, 3);
  const choice = city.choices[0];
  assert.deepEqual(
    collectionBonusIndicators(choice, [{ ...choice, times: 2 }], city),
    city.collectionBonuses,
  );
  const selections = [{ ...choice, times: 3 }];
  const preview = query(game, seat, selections);
  assert.deepEqual(preview.total, { food: 3, ideas: 1, culture_tokens: 1 });
  assert.deepEqual(preview.waste, { food: 3 });
  const after = JSON.parse(engine.tryMove(serialize(game), serialize(preview.action), seat));
  assert.deepEqual(after.players[seat].resources, preview.after);
  assert.deepEqual(collectionBonusIndicators(choice, selections, city), city.collectionBonuses);
});

test('Pottery preview requires its research, its civilization, and an unused ability', async () => {
  for (const missing of ['Storage', 'Japan', 'unused']) {
    const { game, seat, p } = await setup();
    if (missing === 'Storage') p.advances = p.advances.filter((a: string) => a !== 'Storage');
    if (missing === 'Japan') p.civilization = 'Rome';
    if (missing === 'unused') p.event_info = { Pottery: 'used' };
    const city = view(game, seat).cities[0];
    const selections = city.choices.slice(0, 3).map((c) => ({ ...c, times: 1 }));
    assert.deepEqual(city.collectionBonuses, [], missing);
    assert(!query(game, seat, selections).total.culture_tokens, missing);
  }
});

test('storage markers show prospective waste and only the overflowing part of the selected food', async () => {
  const { game, seat, p } = await setup();
  p.resources = { food: 6 };
  const city = view(game, seat).cities[0];
  const selected = city.choices.slice(0, 3).map((c) => ({ ...c, times: 1 }));
  const waste = (choice: Selection, selection: Selection[]) =>
    collectionStorageWaste(choice, selection, city, p.resources, p.resource_limit);
  assert.deepEqual(waste(selected[0], []), {}, 'One food still fits');
  assert.deepEqual(waste(selected[1], selected.slice(0, 1)), { food: 1 });
  assert.deepEqual(
    selected.map((c) => waste(c, selected)),
    [{}, { food: 1 }, { food: 1 }],
  );
  const preview = query(game, seat, selected);
  assert.deepEqual(preview.waste, { food: 2 });
  assert.equal(preview.total.culture_tokens, 1, 'Wasted food still triggers Pottery');
  assert.deepEqual(
    collectionStorageWaste(
      selected[2],
      selected.slice(0, 2),
      { ...city, capacity: 2 },
      p.resources,
      p.resource_limit,
    ),
    {},
  );
  p.resources.food = 7;
  assert.deepEqual(waste(selected[0], []), { food: 1 });
  p.resources.food = 4;
  assert.deepEqual(waste(selected[2], selected), {}, 'Exactly at capacity is not waste');
  const wood = { position: 'X1', pile: { wood: 1 }, times: 1 };
  assert.deepEqual(waste(wood, selected.slice(0, 2)), {}, 'Other resources do not inherit food warnings');
});
