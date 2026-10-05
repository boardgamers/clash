import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { officialWonderText, wonderName } from './wonder-names.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('official wonder names preserve saved IDs and appear in private and public views', async () => {
  const names = {
    Colosseum: 'Great Arena',
    Pyramids: 'Great Pyramid',
    GreatGardens: 'Great Gardens',
    GreatLibrary: 'Great Library',
    GreatLighthouse: 'Great Lighthouse',
    GreatMausoleum: 'Great Mausoleum',
    GreatStatue: 'Great Statue',
    GreatWall: 'Great Wall',
  };
  const game = JSON.parse(await engine.init(2, [], {}, 'wonder-names', {}));
  game.players[0].wonder_cards = Object.keys(names);
  const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  assert.deepEqual(Object.fromEntries(view.wonderCards.map((w: any) => [w.id, w.name])), names);
  for (const [id, name] of Object.entries(names)) assert.equal(wonderName(id), name);
  assert.equal(officialWonderText('Colosseum: built Pyramids'), 'Great Arena: built Great Pyramid');
});

test('built wonder effects are public for both players and spectators; private cards stay private', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'public-wonders', {}));
  game.players[0].cities[0].city_pieces = { wonders: ['GreatLighthouse', 'Pyramids'] };
  game.players[0].wonder_cards = ['GreatLibrary'];
  game.players[1].wonder_cards = ['GreatStatue'];
  for (const seat of [0, 1, undefined]) {
    const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat));
    assert.deepEqual(
      view.builtWonders.map((w: any) => w.id),
      ['GreatLighthouse', 'Pyramids'],
    );
    assert.match(view.builtWonders[0].description, /Whenever this city is activated/);
    assert.ok(Math.abs(view.builtWonders[1].builtPoints - 5.1) < 0.00001);
    assert.equal(view.builtWonders[1].ownedPoints, 0);
    assert.deepEqual(
      view.wonderCards.map((w: any) => w.id),
      seat === undefined ? [] : seat === 0 ? ['GreatLibrary'] : ['GreatStatue'],
    );
  }
});

const json = (g: any) => JSON.stringify(g);
const seatOf = (g: any) => engine.currentPlayer(json(g));
const viewOf = (g: any) => JSON.parse(engine.webView(engine.stripSecret(json(g), seatOf(g)), seatOf(g)));
const move = (g: any, a: any) => JSON.parse(engine.tryMove(json(g), JSON.stringify(a), seatOf(g)));
const query = (g: any, q: any) =>
  JSON.parse(engine.webQuery(engine.stripSecret(json(g), seatOf(g)), seatOf(g), JSON.stringify(q)));
async function lighthouseSetup() {
  const g = JSON.parse(await engine.init(2, [], {}, 'lighthouse-rules', {}));
  const seat = seatOf(g),
    p = g.players[seat],
    city = p.cities[0];
  p.civilization = 'Rome';
  p.advances = ['Farming', 'Mining', 'Fishing', 'Myths', 'Tactics'];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  const water = g.map.tiles.find((t: any) => t[1] === 'Unexplored');
  water[1] = 'Water';
  city.city_pieces = { wonders: ['GreatLighthouse'] };
  const secondCity = g.map.tiles.find((t: any) => t[1] === 'Unexplored');
  secondCity[1] = 'Fertile';
  p.cities.push({ position: secondCity[0], mood_state: 'Happy' });
  const thirdCity = g.map.tiles.find((t: any) => t[1] === 'Unexplored');
  thirdCity[1] = 'Fertile';
  p.cities.push({ position: thirdCity[0], mood_state: 'Happy' });
  return { g, seat, city: city.position, water: water[0] };
}
const collectAction = (g: any, city: string) =>
  query(g, {
    kind: 'collect',
    city,
    selections: [{ ...viewOf(g).cities.find((c: any) => c.position === city).choices[0], times: 1 }],
    variant: 'Collect',
  }).action;

test('Great Lighthouse rewards collection, construction and recruitment without another activation or action', async () => {
  for (const kind of ['collect', 'construct', 'recruit']) {
    const { g, seat, city, water } = await lighthouseSetup();
    assert.ok(!viewOf(g).specialActions.some((a: any) => a.name === 'Great Lighthouse'));
    const action =
      kind === 'collect'
        ? collectAction(g, city)
        : kind === 'recruit'
          ? query(g, { kind, city, units: { infantry: 1 }, replaced: [] }).action
          : viewOf(g)
              .cityActions.find((c: any) => c.position === city)
              .buildings.find((b: any) => b.name === 'Temple').choices[0].action;
    const activated = move(g, action);
    assert.match(viewOf(activated).decision.description, /Great Lighthouse/);
    const after = move(JSON.parse(json(activated)), { Response: { SelectPositions: [water] } });
    assert.equal(after.players[seat].cities[0].activations, 1);
    assert.equal(after.players[seat].cities[0].mood_state, 'Happy');
    assert.equal(after.actions_left, activated.actions_left);
    assert.deepEqual(after.players[seat].resources, activated.players[seat].resources);
    assert.equal(after.players[seat].units.filter((u: any) => u.unit_type === 'Ship').length, 1);
    assert.equal(viewOf(after).decision, null);
    const undone = move(after, 'Undo');
    assert.match(viewOf(undone).decision.description, /Great Lighthouse/);
    assert.equal(undone.players[seat].cities[0].activations, 1);
    const redone = move(undone, 'Redo');
    assert.equal(redone.players[seat].units.filter((u: any) => u.unit_type === 'Ship').length, 1);
    assert.equal(viewOf(redone).decision, null);
    const skipped = move(activated, { Response: { SelectPositions: [] } });
    assert.equal(skipped.players[seat].units.filter((u: any) => u.unit_type === 'Ship').length, 0);
    assert.equal(viewOf(skipped).decision, null);
  }
});

test('Great Lighthouse rewards the last angry-city activation, then the city cannot activate again', async () => {
  const { g, seat, city, water } = await lighthouseSetup();
  g.players[seat].cities[0].mood_state = 'Angry';
  g.players[seat].cities[0].activations = 3;
  const action = collectAction(g, city);
  const after = move(move(g, action), { Response: { SelectPositions: [water] } });
  assert.equal(after.players[seat].cities[0].activations, 4);
  assert.equal(after.players[seat].cities[0].angry_activation, true);
  assert.equal(after.actions_left, g.actions_left - 1);
  assert.equal(viewOf(after).decision, null);
  assert.throws(() => move(after, action));
});

test('Great Lighthouse skips the reward for another city, exhausted ship supply or enemy sea spaces', async () => {
  for (const condition of ['other city', 'no ships', 'enemy ships']) {
    const { g, seat, city, water } = await lighthouseSetup();
    let target = city;
    if (condition === 'other city') target = g.players[seat].cities[1].position;
    if (condition === 'no ships') {
      for (let i = 0; i < 4; i++)
        g.players[seat].units.push({
          id: g.players[seat].next_unit_id++,
          position: water,
          unit_type: 'Ship',
        });
    }
    if (condition === 'enemy ships') {
      const enemy = g.players[1 - seat];
      enemy.units.push({ id: enemy.next_unit_id++, position: water, unit_type: 'Ship' });
    }
    const after = move(g, collectAction(g, target));
    assert.equal(viewOf(after).decision, null, condition);
    assert.equal(after.players[seat].units.length, g.players[seat].units.length);
  }
});
