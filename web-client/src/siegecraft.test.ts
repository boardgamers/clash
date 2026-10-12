import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'siegecraft', {})).players.slice(2);
function fixture(attacker = true, defender = true) {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/combat/direct_capture_city_fortress.json', import.meta.url), 'utf8'));
  g.players.push(...structuredClone(npcs));
  if (attacker) g.players[0].advances.push('Siegecraft');
  if (defender) g.players[1].advances.push('Siegecraft');
  return g;
}
const move = (g: any, action: any, seat = 0) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), seat));
const attack = (g: any) => move(g, { Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1' } } });
const siege = (g: any) => g.events?.find((e: any) => e.handler?.origin?.Advance === 'Siegecraft');
test('Siegecraft is never offered to a defender, including after attacker declines', () => {
  assert(!siege(attack(fixture(false))));
  const g = attack(fixture());
  assert.equal(siege(g).player, 0);
  const after = move(g, { Response: { Payment: [{}, {}] } });
  assert(!siege(after));
});
test('attacker can still pay to cancel both fortress abilities', () => {
  const g = attack(fixture());
  const after = move(g, { Response: { Payment: [{ wood: 2 }, { ore: 2 }] } });
  assert(!siege(after));
  assert.equal(after.players[0].resources.wood, g.players[0].resources.wood - 2);
  assert.equal(after.players[0].resources.ore, g.players[0].resources.ore - 2);
  assert.match(JSON.stringify(after), /Cancel Fortress Extra Die/);
  assert.match(JSON.stringify(after), /Cancel Fortress Ignore Hit/);
});
for (const payment of [[{}, {}], [{ gold: 2 }, { gold: 2 }]]) {
  test(`saved defender prompt safely resolves ${JSON.stringify(payment)}`, () => {
    const g = attack(fixture());
    const event = siege(g);
    event.player = 1;
    event.players_used = [0];
    const after = move(g, { Response: { Payment: payment } }, 1);
    assert(!siege(after));
    assert.equal(after.players[1].resources.gold, g.players[1].resources.gold);
    assert.doesNotMatch(JSON.stringify(after), /CancelFortressExtraDie|CancelFortressIgnoreHit|Cancel Fortress Extra Die|Cancel Fortress Ignore Hit/);
    assert.match(JSON.stringify(after), /Siegecraft skipped: only the attacker may use it/);
  });
}
