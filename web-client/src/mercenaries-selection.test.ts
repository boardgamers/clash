import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
for (const hexes of [['B3', 'B3'], ['B3', 'B3', 'A3']]) {
  test(`Mercenaries counts distinct army hexes: ${hexes.join(',')}`, async () => {
    const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/action_cards/mercenaries.json', import.meta.url), 'utf8'));
    g.players[1].units = []; // No Sulla blocking nearby armies.
    g.players[2].units = hexes.map((position, id) => ({ position, id, unit_type: 'Infantry' }));
    g.players[2].next_unit_id = hexes.length;
    const npcs = JSON.parse(await engine.init(2, [], {}, 'mercenary-selection', {})).players;
    g.players.push({ ...npcs.find((p: any) => p.civilization === 'Pirates'), id: 3 });
    let s = engine.tryMove(JSON.stringify(g), JSON.stringify({ Playing: { ActionCard: 13 } }), 0);
    const event = JSON.parse(s).events.at(-1);
    if (hexes.includes('A3')) {
      const request = event.handler.request.SelectPositions;
      assert.deepEqual(request.choices, ['A3', 'B3']);
      assert.equal(request.needed.end, 2);
      s = engine.tryMove(s, JSON.stringify({ Response: { SelectPositions: ['B3'] } }), 0);
    }
    const payment = JSON.parse(s).events.at(-1).handler.request.Payment;
    assert.ok(payment, 'One hex is automatically selected, or one chosen from multiple hexes');
    const paid = engine.tryMove(s, JSON.stringify({ Response: { Payment: [{ ore: 1 }] } }), 0);
    assert.equal(JSON.parse(paid).players[0].resources.ore, g.players[0].resources.ore - 1);
  });
}
