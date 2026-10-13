import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'platform-log', {})).players.slice(2);

test('BGS receives a plain text field for just the new move, without changing history cursors', () => {
  const game = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/advances/collect_free_economy.json', import.meta.url),
      'utf8',
    ),
  );
  for (const npc of npcs)
    if (!game.players.some((p: { civilization: string }) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  const before = JSON.stringify(game),
    start = engine.logLength(before);
  const action = {
    Playing: {
      Collect: {
        city_position: 'C2',
        collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
        action_type: 'Collect',
      },
    },
  };
  const after = engine.tryMove(before, JSON.stringify(action), 0);
  const log = engine.logSlice(after, { start });
  assert.equal(log.length, 1);
  assert.equal(typeof log[0].text, 'string');
  assert.match(log[0].text, /Gained 1 ore/);
  assert.ok(log[0].text.startsWith(`${game.players[0].civilization}: `));
  assert.equal(log[0].player, 0);
  assert.deepEqual(log[0].action, action);
  assert.equal(engine.logLength(after), start + 1);
  const undone = engine.tryMove(after, JSON.stringify('Undo'), 0);
  assert.deepEqual(engine.logSlice(undone, { start }), []);
  assert.equal(engine.logLength(undone), start);
});
