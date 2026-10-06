import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'guide-fixtures', {})).players.slice(2);
function fixture(name: string) {
  const g = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!g.players.some((p: any) => p.civilization === npc.civilization))
      g.players.push({ ...npc, id: g.players.length, cities: [], units: [] });
  return g;
}
const view = (g: any, seat?: number) =>
  JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), seat), seat));
const query = (g: any, action: any) =>
  JSON.parse(
    engine.webQuery(
      engine.stripSecret(JSON.stringify(g), 0),
      0,
      JSON.stringify({ kind: 'researchPlan', action }),
    ),
  );

test('barbarian guide is public, targets the incident player, and never changes the save', () => {
  const g = fixture('incidents/barbarians_move');
  const before = JSON.stringify(g);
  const guides = view(g, 0).barbarianGuide;
  assert.deepEqual(view(g, 1).barbarianGuide, guides);
  assert.deepEqual(view(g).barbarianGuide, guides);
  assert.ok(guides[0].moves.length);
  assert.deepEqual(guides[0].reinforce, ['B3']);
  assert.equal(guides[1].moves.length, 0, 'an army cannot march towards the other player from this position');
  for (const arrow of guides[0].moves) {
    assert.ok(
      g.players
        .find((p: any) => p.civilization === 'Barbarians')
        .units.some((u: any) => u.position === arrow.from),
    );
    assert.ok(g.map.tiles.some(([pos, terrain]: any) => pos === arrow.to && terrain !== 'Water'));
    assert.notEqual(arrow.from, arrow.to);
  }
  assert.equal(JSON.stringify(g), before);
});

test('saved single-city Myths payments describe one city and still offer zero or one token', () => {
  const g = fixture('incidents/earthquake/flood');
  g.players[0].advances.push('Myths');
  const after = JSON.parse(
    engine.tryMove(
      JSON.stringify(g),
      JSON.stringify({ Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } }),
      0,
    ),
  );
  const request = after.events.at(-1).handler.request.Payment[0];
  request.name = 'You may pay 1 mood token for each city to avoid reducing the mood';
  const field = view(after, 0).decision.fields[0];
  assert.match(field.name, /one affected city/);
  assert.deepEqual(field.choices, [{}, { mood_tokens: 1 }]);
});

test('Free Education quote requires a remaining idea and accounts for Philosophy reward', () => {
  const g = fixture('advances/free_education');
  g.players[0].resources.ideas = 2;
  const irrigation = view(g, 0).advances.find((a: any) => a.id === 'Irrigation').action;
  assert.deepEqual(query(g, irrigation), { eligible: true, affordable: false });
  g.players[0].resources.ideas = 3;
  assert.deepEqual(query(g, irrigation), { eligible: true, affordable: true, combined: true });
  g.players[0].resources.ideas = 2;
  const philosophy = view(g, 0).advances.find((a: any) => a.id === 'Philosophy').action;
  assert.ok(philosophy);
  const before = JSON.stringify(g);
  assert.deepEqual(query(g, philosophy), { eligible: true, affordable: true, combined: true });
  assert.equal(JSON.stringify(g), before);
});
