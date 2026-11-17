import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
const apply = (raw: string, action: unknown, player = engine.currentPlayer(raw)) =>
  engine.tryMove(raw, JSON.stringify(action), player);
const view = (raw: string, player: number) =>
  JSON.parse(engine.webView(engine.stripSecret(raw, player), player));
const actions = (raw: string) =>
  JSON.parse(raw).log.flatMap((a: any) =>
    a.rounds.flatMap((r: any) => r.turns.flatMap((t: any) => t.actions ?? [])),
  );
async function battle() {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/tactics_cards/peltasts.json', import.meta.url),
      'utf8',
    ),
  );
  g.players = [
    ...g.players.slice(0, 2),
    ...JSON.parse(await engine.init(2, [], {}, 'tactics-queue', {})).players.slice(2),
  ];
  for (const p of g.players.slice(0, 2)) {
    p.advances.push('SteelWeapons');
    p.resources.ore = 4;
  }
  g.players[0].action_cards = [3, 6, 11];
  g.players[1].action_cards = [4, 7, 23];
  return apply(
    JSON.stringify(g),
    { Movement: { Move: { units: [0, 1, 2], destination: 'C1', payment: {} } } },
    0,
  );
}
test('actual packed engine queues privately after Steel; consumes once as a normal response and replays', async () => {
  let raw = await battle();
  assert(!view(raw, 0).earlyChoices.some((o: any) => o.key.endsWith('/tactics')));
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } });
  const before = raw;
  const offer = view(raw, 0).earlyChoices.find((o: any) => o.key.endsWith('/tactics'));
  assert(offer);
  const command = JSON.stringify({
    QueueChoice: { key: offer.key, response: { SelectHandCards: [{ ActionCard: 3 }] } },
  });
  assert(engine.canMoveOutOfTurn(raw, command, 0));
  raw = engine.tryMove(raw, command, 0);
  assert(engine.isLiveUpdate(raw));
  for (const seat of [1, undefined])
    assert.deepEqual(JSON.parse(engine.stripSecret(raw, seat)), JSON.parse(engine.stripSecret(before, seat)));
  for (const field of ['players', 'events', 'rng', 'log', 'board_history', 'time_increments'])
    assert.deepEqual(JSON.parse(raw)[field], JSON.parse(before)[field], field);
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } });
  assert.equal(engine.currentPlayer(raw), 1);
  assert.equal(JSON.parse(raw).early_choices, undefined);
  assert(!JSON.parse(raw).players[0].action_cards.includes(3));
  assert.equal(view(raw, 1).decision.opponentTactics, 'played');
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 4 }] } });
  const added = actions(raw)
    .slice(actions(before).length)
    .map((a: any) => a.action);
  assert.equal(added.filter((a: any) => a.Response?.SelectHandCards?.[0]?.ActionCard === 3).length, 1);
  let replay = before;
  for (const action of added) if (action !== 'StartTurn') replay = apply(replay, action);
  for (const field of ['players', 'events', 'rng', 'time_increments'])
    assert.deepEqual(JSON.parse(replay)[field], JSON.parse(raw)[field], field);
});
