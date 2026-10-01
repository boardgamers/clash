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
