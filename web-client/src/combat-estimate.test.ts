import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { addEstimate, percentages } from './combat-estimate.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
function fixture(name: string) {
  return readFileSync(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8');
}
function attack(raw: string, units: number[], destination: string) {
  const choices = JSON.parse(
    engine.webQuery(raw, 0, JSON.stringify({ kind: 'movement', units })),
  ).destinations;
  return choices.find((d: any) => d.position === destination && d.attack).action;
}
function estimate(raw: string, action: unknown, extra: object = {}) {
  return JSON.parse(
    engine.webQuery(raw, 0, JSON.stringify({ kind: 'combatEstimate', action, samples: 64, ...extra })),
  );
}
test('simulation is deterministic, independent of live dice and never changes the state', () => {
  const raw = engine.stripSecret(fixture('direct_capture_city_metallurgy'), 0);
  const action = attack(raw, [0, 1, 2, 3], 'C1');
  const before = engine.tryMove(raw, JSON.stringify(action), 0);
  const result = estimate(raw, action);
  assert.equal(result.wins + result.draws + result.losses, 64);
  assert.ok(result.wins > 50);
  assert.deepEqual(estimate(raw, action), result);
  assert.equal(engine.tryMove(raw, JSON.stringify(action), 0), before);
  const changed = JSON.parse(raw);
  changed.seed = '999';
  changed.dice_roll_outcomes = [0, 0, 0, 0];
  changed.dice_roll_log = [11, 11];
  assert.deepEqual(estimate(JSON.stringify(changed), action), result);
});
test('Siegecraft alternatives run full combat on filtered hands and account for both costs', () => {
  const raw = engine.stripSecret(fixture('combat_all_modifiers'), 0);
  const action = attack(raw, [0, 1, 2, 3, 4, 5], 'C1');
  const base = estimate(raw, action);
  const both = estimate(raw, action, { siege: 3 });
  assert.equal(base.samples, 64);
  assert.equal(both.samples, 64);
  assert.ok(Object.values(both.cost).reduce((a: number, b: any) => a + b, 0) >= 4);
});
test('naval estimates handle passengers and do not use them as fighting dice', () => {
  const loaded = JSON.parse(engine.stripSecret(fixture('ship_combat'), 0));
  loaded.players[1].units[0].carried_units = [
    { id: 2, unit_type: 'Infantry' },
    { id: 3, unit_type: 'Infantry' },
  ];
  const raw = JSON.stringify(loaded);
  const action = attack(raw, [7, 8], 'D2');
  const result = estimate(raw, action);
  assert.equal(result.wins + result.draws + result.losses, 64);
  const bare = JSON.parse(raw);
  for (const p of bare.players) {
    p.units = (p.units ?? []).filter((u: any) => u.carrier_id == null);
    for (const u of p.units) delete u.carried_units;
  }
  const without = estimate(JSON.stringify(bare), action);
  assert.deepEqual([result.wins, result.draws, result.losses], [without.wins, without.draws, without.losses]);
  assert.ok(result.ownLosses > without.ownLosses);
  assert.ok(result.enemyLosses > without.enemyLosses);
});
test('batch totals and displayed percentages preserve all outcomes', () => {
  const e = { samples: 3, wins: 1, draws: 1, losses: 1, ownLosses: 4, enemyLosses: 2, cost: {} };
  assert.equal(
    percentages(e).reduce((a, b) => a + b),
    100,
  );
  assert.deepEqual(addEstimate(e, e), {
    ...e,
    samples: 6,
    wins: 2,
    draws: 2,
    losses: 2,
    ownLosses: 8,
    enemyLosses: 4,
  });
});

test('one ship against one ship agrees with exact 40/20/40 odds', () => {
  const game = JSON.parse(fixture('ship_combat'));
  for (const p of game.players) {
    p.advances = ['Farming', 'Tactics'];
    p.special_advances = [];
    p.units = (p.units ?? []).filter((u: any) => u.unit_type === 'Ship').slice(0, 1);
  }
  game.state = 'Playing';
  game.events = [];
  game.actions_left = 3;
  for (const p of game.players) for (const u of p.units) u.movement_restrictions = [];
  const raw = engine.stripSecret(JSON.stringify(game), 0);
  const units = game.players[0].units.map((u: any) => u.id);
  const action = attack(raw, units, 'D2');
  let total;
  for (let offset = 0; offset < 2048; offset += 64)
    total = addEstimate(total, estimate(raw, action, { offset }));
  assert.ok(Math.abs(total!.wins / 2048 - 0.4) < 0.04, JSON.stringify(total));
  assert.ok(Math.abs(total!.draws / 2048 - 0.2) < 0.04, JSON.stringify(total));
  assert.ok(Math.abs(total!.losses / 2048 - 0.4) < 0.04, JSON.stringify(total));
});

test('full and filtered states produce identical odds without reading hidden opponent cards', () => {
  const raw = fixture('combat_all_modifiers');
  const visible = engine.stripSecret(raw, 0);
  const action = attack(visible, [0, 1, 2, 3, 4, 5], 'C1');
  assert.deepEqual(estimate(raw, action), estimate(visible, action));
});

test('selected Steel Weapons and tactics responses are simulated before confirming', () => {
  let raw = fixture('combat_all_modifiers');
  raw = engine.tryMove(raw, JSON.stringify(attack(raw, [0, 1, 2, 3, 4, 5], 'C1')), 0);
  const paid = { Response: { Payment: [{ ore: 1 }] } };
  const skipped = { Response: { Payment: [{}] } };
  const withSteel = estimate(engine.stripSecret(raw, 0), paid);
  const withoutSteel = estimate(engine.stripSecret(raw, 0), skipped);
  assert.notDeepEqual(withSteel, withoutSteel);
  raw = engine.tryMove(raw, JSON.stringify(paid), 0);
  raw = engine.tryMove(raw, JSON.stringify({ Response: { Payment: [{}, {}] } }), 0);
  raw = engine.tryMove(raw, JSON.stringify(paid), 1);
  const visible = engine.stripSecret(raw, 0);
  const withTactic = estimate(visible, { Response: { SelectHandCards: [{ ActionCard: 1 }] } });
  const withoutTactic = estimate(visible, { Response: { SelectHandCards: [] } });
  assert.equal(withTactic.samples, 64);
  assert.notDeepEqual(withTactic, withoutTactic);
});

test('unaffordable Siegecraft is unavailable, never presented as a free cancellation', () => {
  const game = JSON.parse(fixture('combat_all_modifiers'));
  game.players[0].resources = {};
  const raw = engine.stripSecret(JSON.stringify(game), 0);
  const action = attack(raw, [0, 1, 2, 3, 4, 5], 'C1');
  assert.equal(estimate(raw, action).samples, 64);
  assert.throws(() => estimate(raw, action, { siege: 3 }), /Cannot afford/);
});

test('pirate battle estimates resolve round rewards, including zero gold, without changing the game', () => {
  const game = JSON.parse(fixture('ship_combat'));
  game.players[2].units = game.players[1].units;
  game.players[1].units = [];
  game.players[0].action_cards = [6];
  const raw = JSON.stringify(game);
  const action = attack(raw, [7, 8], 'D2');
  const before = engine.tryMove(raw, JSON.stringify(action), 0);
  // This batch includes no-hit rounds that previously failed on the empty reward.
  const result = estimate(raw, action);
  assert.equal(result.wins + result.draws + result.losses, 64);
  assert.deepEqual(estimate(raw, action), result);
  assert.equal(engine.tryMove(raw, JSON.stringify(action), 0), before);
  const view = JSON.parse(engine.webView(before, 0));
  assert.equal(view.decision.name, 'Tactics');
  const fromTactics = estimate(before, { Response: { SelectHandCards: [] } });
  assert.equal(fromTactics.wins + fromTactics.draws + fromTactics.losses, 64);
});
