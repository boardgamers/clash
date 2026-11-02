import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = readFileSync(
  new URL('../../server/tests/fixtures/tribal_allies_public_board.json', import.meta.url), 'utf8',
);
const move = (state: string, action: unknown) => engine.tryMove(state, JSON.stringify(action), 0);
const query = (state: string, input: unknown) => JSON.parse(engine.webQuery(
  engine.stripSecret(state, 0), 0, JSON.stringify(input),
));
const view = (state: string) => JSON.parse(engine.webView(engine.stripSecret(state, 0), 0));

test('Tribal Allies permits Barbarian settlement adjacency but still excludes Maya city adjacency', () => {
  // Sanitized board reconstruction, not a full historical state or live-game move.
  for (const fortress of [true, false]) {
    const game = JSON.parse(fixture);
    if (!fortress) delete game.players[0].cities.find((c: any) => c.position === 'C4').city_pieces.fortress;
    const pending = move(JSON.stringify(game), { Playing: { Custom: { action: 'TribalAllies' } } });
    const decision = view(pending).decision;
    assert.match(decision.description, /Place a Barbarian city and Infantry/);
    assert.equal(decision.min, 1);
    assert.deepEqual(decision.options.map((o: any) => o.position).sort(), ['B4', 'C2', 'C3', 'E2']);
    assert.throws(() => query(pending, { kind: 'decision', values: ['C5'] }),
      'C5 borders the opposing human city C6');
    for (const position of ['B4', 'C3']) {
      const placement = query(pending, { kind: 'decision', values: [position] });
      const placed = JSON.parse(move(pending, placement.action));
      const barbarians = placed.players.find((p: any) => p.civilization === 'Barbarians');
      assert.ok(barbarians.cities.some((c: any) => c.position === position));
      assert.deepEqual(barbarians.units.filter((u: any) => u.position === position)
        .map((u: any) => u.unit_type), ['Infantry']);
      assert.equal(placed.actions_left, 0);
    }
  }
});
