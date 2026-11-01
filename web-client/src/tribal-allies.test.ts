import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fresh = JSON.parse(await engine.init(2, [], {}, 'tribal-allies', {}));
const move = (g: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
const request = (g: any) => g.events?.at(-1)?.handler?.request;
function start() {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/barbarians_move.json', import.meta.url), 'utf8'));
  g.players[0].civilization = 'Celts';
  g.players[0].advances.push('Draft');
  g.players[1].units = g.players[1].units.filter((u: any) => !u.unit_type.Leader);
  g.players.push(structuredClone(fresh.players[3]));
  return move(g, { Response: { SelectAdvance: 'Storage' } });
}

test('Tribal Allies selects armies before charging anything; skipping is free', () => {
  const g = start();
  assert.deepEqual(request(g).SelectPositions.choices, ['A3', 'B3']);
  assert.equal(g.players[0].resources.food, 1);
  const after = move(g, { Response: { SelectPositions: [] } });
  assert.equal(after.players[0].resources.food, 1);
  assert(!JSON.stringify(after.events).includes('StopBarbarianMovement'));
});

for (const payment of [{ food: 1 }, { gold: 1 }]) {
  test(`Tribal Allies stops selected armies for one total payment: ${JSON.stringify(payment)}`, () => {
    let g = start();
    const before = structuredClone(g.players[0].resources);
    g = move(g, { Response: { SelectPositions: ['A3', 'B3'] } });
    assert.deepEqual(g.players[0].resources, before);
    assert(request(g).Payment);
    g = move(g, { Response: { Payment: [payment] } });
    assert(g.players[2].units.filter((u: any) => [1, 4].includes(u.id)).every((u: any) => u.position === 'A3'));
    assert(g.players[2].units.filter((u: any) => [0, 2].includes(u.id)).every((u: any) => u.position === 'B3'));
    const key = Object.keys(payment)[0];
    assert.equal(g.players[0].resources[key] ?? 0, before[key] - 1);
    assert(!g.players[0].event_info?.['Tribal Allies armies']);
  });
}

test('canceling payment stops no armies and spends nothing', () => {
  let g = start();
  const before = structuredClone(g.players[0].resources);
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  g = move(g, { Response: { Payment: [{}] } });
  assert.deepEqual(g.players[0].resources, before);
  assert(!g.players[0].event_info?.['Tribal Allies armies']);
  assert(!JSON.stringify(g.events).includes('StopBarbarianMovement'));
});

test('only selected armies stop', () => {
  let g = start();
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  g = move(g, { Response: { Payment: [{ food: 1 }] } });
  assert(g.players[2].units.filter((u: any) => [1, 4].includes(u.id)).every((u: any) => u.position === 'A3'));
  assert(g.players[2].units.filter((u: any) => [0, 2].includes(u.id)).every((u: any) => u.position !== 'B3'));
});

test('a payment pending from the previous flow still leads to an army selection', () => {
  let g = move(start(), { Response: { SelectPositions: ['A3'] } });
  delete g.players[0].event_info['Tribal Allies armies'];
  g = move(g, { Response: { Payment: [{ food: 1 }] } });
  assert.equal(request(g).SelectPositions.needed.start, 1);
  assert.throws(() => move(g, { Response: { SelectPositions: [] } }));
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  assert(g.players[2].units.filter((u: any) => [1, 4].includes(u.id)).every((u: any) => u.position === 'A3'));
});
