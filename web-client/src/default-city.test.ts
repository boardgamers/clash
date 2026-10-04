import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { defaultCity } from './default-city.ts';
import type { CityView, View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

const city = (position: string, size: number, activations = 0) =>
  ({ position, size, activations, reason: null }) as CityView;
const offers = (position: string, build: boolean, recruit: boolean) => ({
  position,
  buildings: [{ owned: false, choices: build ? [{ action: {} }] : [] }],
  recruits: [{ reason: recruit ? null : 'Unavailable', available: 1 }],
});

test('defaults prioritize unactivated usable cities, then yield/size, with stable ties and inspection fallback', () => {
  const view = {
    cities: [city('A1', 2), city('B1', 3), city('C1', 5, 1), city('D1', 4)],
    cityActions: [
      offers('A1', true, true),
      offers('B1', true, false),
      offers('C1', true, true),
      offers('D1', false, true),
    ],
  } as View;
  assert.equal(defaultCity(view, 'build', 'C1'), 'B1');
  assert.equal(defaultCity(view, 'recruit', 'C1'), 'D1');
  const yieldOf = [
    { position: 'A1', amount: 5 },
    { position: 'B1', amount: 4 },
    { position: 'C1', amount: 8 },
  ];
  assert.equal(defaultCity(view, 'collect', 'C1', yieldOf), 'A1', 'bonuses can beat a larger city');
  view.cities[0].activations = 1;
  assert.equal(defaultCity(view, 'collect', 'A1', yieldOf), 'B1', 'preserve mood before optimizing yield');
  view.cities[1].activations = 1;
  assert.equal(
    defaultCity(view, 'collect', 'A1', yieldOf),
    'C1',
    'still usable when every city is activated',
  );
  assert.equal(
    defaultCity({ ...view, cityActions: [] }, 'build', 'B1'),
    'B1',
    'keep reference browsing useful',
  );
  assert.equal(defaultCity(null, 'build', null), null);
  assert.equal(
    defaultCity({ ...view, cities: [city('A1', 2), city('B1', 2)] }, 'collect', 'B1', [
      { position: 'A1', amount: 3 },
      { position: 'B1', amount: 3 },
    ]),
    'B1',
  );
});

async function setup(civilization = 'Rome') {
  const g = JSON.parse(await engine.init(2, [], {}, 'city-defaults', {}));
  const seat = engine.currentPlayer(JSON.stringify(g)),
    p = g.players[seat];
  p.civilization = civilization;
  p.advances = ['Farming', 'Mining', 'Storage', 'Writing', 'PublicEducation', 'Irrigation'];
  p.cities = [{ position: 'C2', mood_state: 'Happy', city_pieces: { academy: seat } }];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7 };
  p.units = [];
  g.map.tiles = [
    ['C2', 'Fertile'],
    ['B1', 'Forest'],
    ['B2', 'Fertile'],
    ['C1', 'Mountain'],
    ['D1', 'Forest'],
    ['D2', 'Fertile'],
  ];
  return { g, seat, p };
}
const potential = (g: any, seat: number) =>
  JSON.parse(
    engine.webQuery(
      JSON.stringify(g),
      seat,
      JSON.stringify({ kind: 'collectionPotential', variant: 'Collect' }),
    ),
  );

test('collection potential includes happy capacity and Academy/Public Education, without spending its bonus', async () => {
  const { g, seat, p } = await setup();
  const before = JSON.stringify(g);
  assert.equal(potential(g, seat)[0].amount, 4, '3 tiles plus 1 idea');
  assert.equal(JSON.stringify(g), before, 'ranking is read-only');
  p.cities[0].mood_state = 'Neutral';
  assert.equal(potential(g, seat)[0].amount, 3);
  p.event_info = { PublicEducation: 'used' };
  assert.equal(potential(g, seat)[0].amount, 2, 'spent once-per-turn bonuses do not count');
  p.cities[0].activations = 1;
  p.cities[0].angry_activation = true;
  p.cities[0].mood_state = 'Angry';
  assert.deepEqual(potential(g, seat), [], 'cannot activate this city');
});

test('collection potential respects scarce terrain and focused collection; Pottery is included', async () => {
  const { g, seat } = await setup('Japan');
  g.map.tiles = [['C2', 'Fertile']];
  assert.equal(potential(g, seat)[0].amount, 2, 'one accessible tile plus Education');
  g.permanent_effects = [{ Collect: 'ProductionFocus' }];
  assert.equal(
    potential(g, seat)[0].amount,
    5,
    'three food plus Education and Pottery, even at full storage',
  );
});

test('collection potential matches exhaustive legal previews with Canals and repeatable mixed resources', async () => {
  const { g, seat, p } = await setup('Babylonia');
  g.permanent_effects = [{ Collect: 'ProductionFocus' }];
  p.advances.push('Engineering');
  g.map.tiles = [
    ['C2', 'Fertile'],
    ['C1', 'Forest'],
  ];
  const raw = JSON.stringify(g),
    view = JSON.parse(engine.webView(raw, seat)),
    city = view.cities[0];
  let best = 0;
  function enumerate(index: number, selected: any[], used: number) {
    if (index === city.choices.length) {
      if (!used) return;
      try {
        const preview = JSON.parse(
          engine.webQuery(
            raw,
            seat,
            JSON.stringify({ kind: 'collect', city: 'C2', variant: 'Collect', selections: selected }),
          ),
        );
        best = Math.max(
          best,
          Object.values(preview.total).reduce<number>((a, b) => a + (b as number), 0),
        );
      } catch {}
      return;
    }
    const choice = city.choices[index];
    const onTile = selected.filter((c) => c.position === choice.position).reduce((n, c) => n + c.times, 0);
    for (let times = 0; times <= Math.min(city.maxPerTile - onTile, city.capacity - used); times++)
      enumerate(index + 1, times ? [...selected, { ...choice, times }] : selected, used + times);
  }
  enumerate(0, [], 0);
  assert.ok(best >= 5);
  assert.equal(potential(g, seat)[0].amount, best);
});

test('collection potential counts Rice Cultivation once per eligible tile and honors Husbandry range limits', async () => {
  const { g, seat, p } = await setup('China');
  p.units = [
    { id: 0, unit_type: 'Settler', position: 'B2' },
    { id: 1, unit_type: 'Settler', position: 'D2' },
  ];
  p.next_unit_id = 2;
  assert.equal(potential(g, seat)[0].amount, 6, '3 tiles, 2 rice bonuses, 1 idea');
  p.civilization = 'Rome';
  p.advances = p.advances.filter((a: string) => a !== 'Irrigation');
  p.advances.push('Husbandry');
  p.units = [];
  g.map.tiles = [
    ['C2', 'Barren'],
    ['B1', 'Barren'],
    ['B2', 'Barren'],
    ['C1', 'Forest'],
    ['D1', 'Barren'],
    ['D2', 'Barren'],
    ['A1', 'Fertile'],
    ['A2', 'Fertile'],
    ['E2', 'Fertile'],
  ];
  assert.equal(potential(g, seat)[0].amount, 3, 'one adjacent and one distant tile, plus Education');
  p.advances.push('Roads');
  assert.equal(potential(g, seat)[0].amount, 4, 'Roads permits a second distant tile');
});
