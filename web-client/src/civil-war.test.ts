import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'civil-war', {})).players.slice(2);
function fixture(mood: string, myths = false) {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/incidents/civil_war/civil_war.json', import.meta.url),
      'utf8',
    ),
  );
  g.players.push(...structuredClone(npcs));
  const p = g.players[0];
  p.cities = [{ position: 'C2', mood_state: mood }];
  p.units = [{ id: 0, position: 'C2', unit_type: 'Infantry' }];
  p.advances = p.advances.filter((a: string) => a !== 'Myths');
  if (myths) p.advances.push('Myths');
  return JSON.stringify(g);
}
function advance(g: string) {
  return engine.tryMove(
    g,
    JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
    0,
  );
}
for (const mood of ['Happy', 'Neutral', 'Angry'])
  test(`Civil War resolves once from ${mood}`, () => {
    const after = JSON.parse(advance(fixture(mood)));
    assert.equal(after.players[0].cities[0].mood_state, mood === 'Happy' ? 'Neutral' : 'Angry');
    assert.equal(after.players[0].units?.length ?? 0, mood === 'Happy' ? 1 : 0);
    assert.equal(after.events?.length ?? 0, 0);
  });
for (const mood of ['Happy', 'Neutral'])
  test(`Myths protects Civil War mood but not infantry in ${mood} city`, () => {
    let raw = advance(fixture(mood, true));
    assert(JSON.parse(raw).events.at(-1).handler.request.Payment);
    raw = engine.tryMove(raw, JSON.stringify({ Response: { Payment: [{ mood_tokens: 1 }] } }), 0);
    const after = JSON.parse(raw);
    assert.equal(after.players[0].cities[0].mood_state, mood);
    assert.equal(after.players[0].units?.length ?? 0, mood === 'Happy' ? 1 : 0);
  });
