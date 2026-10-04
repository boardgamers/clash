import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'terrain-ui', {})).players.slice(2);
function fixture(terrain: unknown) {
  const g = JSON.parse(
    readFileSync(new URL('../../server/tests/test_games/movement/movement.json', import.meta.url), 'utf8'),
  );
  g.players.push(...structuredClone(npcs));
  g.players[0].advances.push('Tactics');
  g.players[1].cities[0].position = 'A3';
  g.players[1].units = [];
  g.map.tiles.find((t: any) => t[0] === 'C1')[1] = terrain;
  return g;
}
function quote(g: any, units = [0]) {
  return JSON.parse(
    engine.webQuery(engine.stripSecret(JSON.stringify(g), 0), 0, JSON.stringify({ kind: 'movement', units })),
  ).destinations.find((d: any) => d.position === 'C1');
}
for (const terrain of ['Forest', 'Mountain'])
  test(`${terrain} movement warning matches the resulting unit restriction`, () => {
    const g = fixture(terrain),
      option = quote(g);
    assert.match(
      option.terrainNotes.join(' '),
      terrain === 'Forest' ? /cannot (?:make a later )?attack/ : /cannot move again/,
    );
    const moved = JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(option.action), 0));
    assert(moved.players[0].units.find((u: any) => u.id === 0).movement_restrictions.includes(terrain));
    g.players[0].advances.push('Roads');
    assert.match(quote(g).terrainNotes.join(' '), /Roads route/);
  });
test('forest settlers and exhausted mountains do not promise restrictions that will not apply', () => {
  const g = fixture('Forest');
  assert.match(quote(g, [5]).terrainNotes.join(' '), /No forest movement restriction/);
  const exhausted = fixture({ Exhausted: 'Mountain' });
  assert.deepEqual(quote(exhausted).terrainNotes, []);
});
test('Hannibal and Terracing show their mountain exceptions before moving', () => {
  const g = fixture('Mountain');
  g.players[0].civilization = 'Carthage';
  g.players[0].units.push({ id: 8, unit_type: { Leader: 'Hannibal' }, position: 'C2' });
  const hannibal = quote(g, [3, 8]);
  assert.match(hannibal.terrainNotes.join(' '), /Hannibal.*ignores/);
  const after = JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(hannibal.action), 0));
  assert(
    after.players[0].units
      .filter((u: any) => [3, 8].includes(u.id))
      .every((u: any) => !u.movement_restrictions?.includes('Mountain')),
  );
  const maya = fixture('Mountain');
  maya.players[0].civilization = 'Maya';
  maya.players[0].advances.push('Irrigation');
  assert.match(quote(maya, [5]).terrainNotes.join(' '), /Terracing/);
  assert.doesNotMatch(quote(maya, [5]).terrainNotes.join(' '), /must stop/);
  assert.match(quote(maya, [0, 5]).terrainNotes.join(' '), /Other units must stop/);
});

test('Great Gardens warning appears only when an opponent has built it', () => {
  const g = fixture('Fertile');
  g.players[0].units[0].movement_restrictions = ['Fertile'];
  const notes = () =>
    JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0)).units.find((u: any) => u.id === 0)
      .movementNotes;
  assert.deepEqual(notes(), []);
  g.players[1].cities[0].city_pieces = { wonders: ['GreatGardens'] };
  assert.match(notes().join(' '), /Great Gardens/);
  g.players[0].cities[0].city_pieces = { wonders: ['GreatGardens'] };
  delete g.players[1].cities[0].city_pieces;
  assert.deepEqual(notes(), [], 'own Great Gardens does not restrict attacks');
});

test('leaving mountains uses this group move but does not impose a turn-long stop', () => {
  const g = fixture('Fertile');
  g.map.tiles.find((t: any) => t[0] === 'C2')[1] = 'Mountain';
  g.actions_left = 2;
  const moved = engine.tryMove(JSON.stringify(g), JSON.stringify(quote(g).action), 0);
  const view = JSON.parse(engine.webView(engine.stripSecret(moved, 0), 0));
  const notes = view.units.find((u: any) => u.id === 0).movementNotes.join(' ');
  assert.match(notes, /Already moved in this Move action/);
  assert.doesNotMatch(notes, /Mountain|Great Gardens/);
  const destinations = (raw: string) =>
    JSON.parse(
      engine.webQuery(engine.stripSecret(raw, 0), 0, JSON.stringify({ kind: 'movement', units: [0] })),
    ).destinations;
  assert.deepEqual(destinations(moved), []);
  const nextAction = engine.tryMove(moved, JSON.stringify(view.stopMovement), 0);
  assert(destinations(nextAction).length > 0, 'a new Move action can move the unit again');
});

test('a newly recruited leader needs Tactics to move, and the unit explains why', async () => {
  let state = await engine.init(2, [], {}, 'leader-needs-tactics', {});
  const g = JSON.parse(state);
  const seat = engine.currentPlayer(state);
  const p = g.players[seat];
  p.civilization = 'Japan';
  p.available_leaders = ['GoToba', 'Jimmu', 'Suiko'];
  p.resources = { mood_tokens: 1, culture_tokens: 1 };
  state = engine.tryMove(
    JSON.stringify(g),
    JSON.stringify({
      Playing: {
        Recruit: {
          city_position: p.cities[0].position,
          units: { leader: 'GoToba' },
          payment: p.resources,
        },
      },
    }),
    seat,
  );
  const after = JSON.parse(state);
  const leader = after.players[seat].units.find((u: any) => typeof u.unit_type === 'object');
  const info = (raw: string) => JSON.parse(engine.webView(engine.stripSecret(raw, seat), seat)).units;
  const routes = (raw: string) =>
    JSON.parse(
      engine.webQuery(
        engine.stripSecret(raw, seat),
        seat,
        JSON.stringify({ kind: 'movement', units: [leader.id] }),
      ),
    ).destinations;
  assert(
    info(state)
      .find((u: any) => u.id === leader.id)
      .movementNotes.includes('Requires Tactics to move'),
  );
  assert.deepEqual(info(state).find((u: any) => u.type === 'Settler').movementNotes, []);
  assert.deepEqual(routes(state), []);
  after.players[seat].advances.push('Tactics');
  state = JSON.stringify(after);
  assert.deepEqual(info(state).find((u: any) => u.id === leader.id).movementNotes, []);
  assert(routes(state).length > 0, 'Recruitment does not prevent movement');
  assert.doesNotThrow(() => engine.tryMove(state, JSON.stringify(routes(state)[0].action), seat));
});
