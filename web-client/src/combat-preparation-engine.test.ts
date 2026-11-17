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
async function battle(legacy = false) {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/tactics_cards/peltasts.json', import.meta.url),
      'utf8',
    ),
  );
  if (legacy) g.asynchronous_combat_from = 999999;
  g.players = [
    ...g.players.slice(0, 2),
    ...JSON.parse(await engine.init(2, [], {}, 'combat-preparation', {})).players.slice(2),
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
test('actual engine completes each player preparation in order and replays ordinary responses', async () => {
  let raw = await battle();
  const before = raw;
  assert.equal(view(raw, 0).decision.name, 'Steel Weapons');
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } });
  assert.equal(engine.currentPlayer(raw), 0);
  assert.equal(view(raw, 0).decision.name, 'Tactics');
  assert(!view(raw, 1).earlyChoices.some((o: any) => o.key.endsWith('/tactics')));
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 3 }] } });
  assert.equal(engine.currentPlayer(raw), 1);
  assert.equal(view(raw, 1).decision.name, 'Steel Weapons');
  assert.equal(JSON.parse(raw).early_choices, undefined);
  assert(!JSON.parse(raw).players[0].action_cards.includes(3));
  for (const seat of [1, undefined]) {
    const hidden = JSON.parse(engine.stripSecret(raw, seat));
    assert.equal(hidden.events.at(-1).event_type.CombatRoundStart.attacker_strength.tactics_card, 0);
    assert(!hidden.action_cards_discarded?.includes(3));
  }
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } });
  assert.equal(engine.currentPlayer(raw), 1);
  assert.equal(view(raw, 1).decision.name, 'Tactics');
  assert.equal(view(raw, 1).decision.opponentTactics, 'played');
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 4 }] } });
  const added = actions(raw).slice(actions(before).length);
  assert.equal(added.filter((a: any) => a.action.Response?.SelectHandCards?.[0]?.ActionCard === 3).length, 1);
  let replay = before;
  for (const item of added) if (item.action !== 'StartTurn') replay = apply(replay, item.action, item.player);
  for (const field of ['players', 'events', 'rng', 'time_increments'])
    assert.deepEqual(JSON.parse(replay)[field], JSON.parse(raw)[field], field);
});

test('legacy format replays both payments before either tactics response', async () => {
  let raw = await battle(true);
  const before = raw;
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } }, 0);
  assert.equal(engine.currentPlayer(raw), 1);
  assert.equal(view(raw, 1).decision.name, 'Steel Weapons');
  raw = apply(raw, { Response: { Payment: [{ ore: 1 }] } }, 1);
  assert.equal(engine.currentPlayer(raw), 0);
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 3 }] } }, 0);
  raw = apply(raw, { Response: { SelectHandCards: [{ ActionCard: 4 }] } }, 1);
  let replay = before;
  for (const item of actions(raw).slice(actions(before).length))
    if (item.action !== 'StartTurn') replay = apply(replay, item.action, item.player);
  for (const field of ['players', 'events', 'rng', 'time_increments'])
    assert.deepEqual(JSON.parse(replay)[field], JSON.parse(raw)[field], field);
});

const historical = readFileSync(
  new URL('../../server/tests/fixtures/legacy_combat_history.json', import.meta.url),
  'utf8',
);
const attackNext = { Movement: { Move: { destination: 'E5', units: [3] } } };
const mode = (raw: string) =>
  JSON.parse(raw).events?.at(-1)?.event_type.CombatRoundStart?.combat.asynchronous_preparation ?? false;
test('engine.replay preserves legacy combats, but full-save and partial-history branches use new preparation', () => {
  const old = engine.replay(historical, {});
  assert.equal(engine.logLength(old), 217);
  const expected = JSON.parse(
    readFileSync(
      new URL('../../server/tests/fixtures/legacy_combat_history.expected.json', import.meta.url),
      'utf8',
    ),
  );
  for (const [key, value] of Object.entries(expected))
    assert.deepEqual(JSON.parse(old)[key], value, `0.4.153 compatibility: ${key}`);
  assert.equal(mode(old), false);
  const finished = apply(old, { Response: { SelectHandCards: [] } }, 1);
  assert.equal(JSON.parse(finished).events?.length ?? 0, 0);
  for (const to of [167, 216]) {
    const branch = engine.replay(historical, { to });
    const next =
      to === 216
        ? apply(branch, attackNext, 0)
        : apply(branch, { Movement: { Move: { destination: 'F7', units: [1] } } }, 1);
    assert.equal(mode(next), true);
    const replayed = engine.replay(next, {});
    for (const field of ['players', 'events', 'rng', 'asynchronous_combat_from'])
      assert.deepEqual(JSON.parse(replayed)[field], JSON.parse(next)[field], field);
  }
  const full = JSON.parse(historical);
  full.log.at(-1).rounds.at(-1).turns.at(-1).actions.pop();
  full.log_index--;
  const replayed = engine.replay(JSON.stringify(full), {});
  assert.equal(mode(apply(replayed, attackNext, 0)), true);
});

test('engine.replay honors an explicit future switch only during reconstruction, not on a new branch', () => {
  const marked = JSON.parse(historical);
  marked.asynchronous_combat_from = 999999;
  assert.equal(mode(engine.replay(JSON.stringify(marked), {})), false);
  const branch = engine.replay(JSON.stringify(marked), { to: 216 });
  assert.equal(mode(apply(branch, attackNext, 0)), true);
});

test('actual engine keeps undo/redo switch points but clamps a replacement branch', async () => {
  const setup = JSON.parse(await engine.init(2, [], {}, 'combat-undo-boundary', {}));
  setup.asynchronous_combat_from = 100;
  const research = { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } };
  let raw = apply(JSON.stringify(setup), research);
  raw = apply(raw, 'Undo');
  assert.equal(JSON.parse(raw).asynchronous_combat_from, 100);
  raw = apply(raw, 'Redo');
  assert.equal(JSON.parse(raw).asynchronous_combat_from, 100);
  raw = apply(raw, 'Undo');
  const next = engine.logLength(raw) + 1;
  raw = apply(raw, research);
  assert.equal(JSON.parse(raw).asynchronous_combat_from, next);
  raw = apply(raw, 'Undo');
  assert.equal(JSON.parse(raw).asynchronous_combat_from, next);
  raw = apply(raw, 'Redo');
  assert.equal(JSON.parse(raw).asynchronous_combat_from, next);
});

test('undo/redo of a newly branched battle cannot restore legacy order', async () => {
  let raw = await battle(true);
  const attack = actions(raw).at(-1).action;
  assert(JSON.parse(raw).events.at(-1).event_type.CombatStart);
  raw = apply(raw, 'Undo', 0);
  raw = apply(raw, attack, 0);
  assert.equal(mode(raw), true);
  const expected = JSON.parse(raw);
  raw = apply(raw, 'Undo', 0);
  raw = apply(raw, 'Redo', 0);
  assert.equal(mode(raw), true);
  for (const field of ['players', 'events', 'rng', 'asynchronous_combat_from'])
    assert.deepEqual(JSON.parse(raw)[field], expected[field], field);
});
