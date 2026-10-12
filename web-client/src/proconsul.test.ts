import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'proconsul', {})).players.slice(2);
const move = (g: any, action: any) => JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
for (const paid of [false, true]) test(`Proconsul ${paid ? 'grants an infantry for one gold' : 'does not grant an infantry when declined'}`, () => {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/civilizations/rome/proconsul.json', import.meta.url), 'utf8'));
  g.players.push(...structuredClone(npcs));
  const pending = move(g, { Movement: { Move: { units: [0], destination: 'D8' } } });
  assert.equal(pending.events.at(-1).handler.origin.LeaderAbility, 'Proconsul');
  const after = move(pending, { Response: { Payment: [paid ? { gold: 1 } : {}] } });
  assert.equal(after.players[0].units.filter((u: any) => u.unit_type === 'Infantry').length, paid ? 1 : 0);
  assert.equal(after.players[0].resources.gold, pending.players[0].resources.gold - (paid ? 1 : 0));
});
