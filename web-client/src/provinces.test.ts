import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
function fixture() {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/civilizations/rome/provinces.json', import.meta.url), 'utf8'));
  g.players[0].advances.push('Voting');
  return g;
}
const move = (g: any, action: any) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
const capture = (g: any) => move(g, { Movement: { Move: { units: [0], destination: 'C1' } } });
const provinces = (g: any) => g.events?.some((e: any) => e.handler?.origin?.SpecialAdvance === 'Provinces');
for (const [payment, mood] of [[{}, 'Neutral'], [{ culture_tokens: 1 }, 'Happy']] as const) {
  test(`Provinces retained city becomes ${mood}`, () => {
    const g = capture(fixture());
    assert(provinces(g));
    const after = move(g, { Response: { Payment: [payment] } });
    assert.equal(after.players[0].cities.find((c: any) => c.position === 'C1').mood_state, mood);
  });
  test(`Provinces obsolete ${mood} choice resolves without charging culture`, () => {
    const g = capture(fixture());
    g.players[0].cities = g.players[0].cities.filter((c: any) => c.position !== 'C1');
    const after = move(g, { Response: { Payment: [payment] } });
    assert(!provinces(after));
    assert.equal(after.players[0].resources.culture_tokens, g.players[0].resources.culture_tokens);
  });
}
test('Provinces does not prompt for a city automatically razed at the city limit', () => {
  const g = fixture();
  g.players[0].cities.push(...['D1', 'D2', 'E1'].map(position => ({ position, mood_state: 'Neutral' })));
  const after = capture(g);
  assert.equal(after.players[0].cities.length, 7);
  assert(!after.players.some((p: any) => p.cities?.some((c: any) => c.position === 'C1')));
  assert(!provinces(after));
});
