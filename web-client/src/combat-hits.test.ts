import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { journal } from './journal.ts';
import { combatJournal } from './combat-journal.ts';
import { battleCues } from './battle-playback.ts';
import type { Game, LoggedAction } from './types.ts';

// Entirely synthetic public battle logs; no production game state.
function gameWith(actions: LoggedAction[]): Game {
  const game: Game = JSON.parse(
    readFileSync(
      new URL(
        '../../server/tests/test_games/combat/remove_casualties_attacker.outcome.json',
        import.meta.url,
      ),
      'utf8',
    ),
  );
  game.players[0].civilization = 'China';
  game.players[1].civilization = 'Celts';
  game.log![0].rounds[0].turns[0].actions = actions;
  game.log_index = actions.length;
  return game;
}
function battle(game: Game) {
  return combatJournal(journal(game)).find((e) => e.combat)!.combat!;
}
function legacy(fighters: number, modifiers: string, hits: number, value = 29): LoggedAction {
  return {
    log: [
      'Combat round 1',
      'Player1: Combat: Attacking with 2 infantry and Vercingetorix',
      `Player2: Combat: Defending with ${fighters} infantry and 2 settlers`,
      `Player1: Combat: Roll 6 for combined combat value of ${value} and gets ${hits} hits against defending units`,
      ...(modifiers ? [`Player2: Combat: Combat modifiers: ${modifiers}`] : []),
    ],
  };
}
for (const fighters of [3, 4])
  test(`29 value vs ${fighters} fighters: fortress cancels a hit before the unit cap`, () => {
    const game = gameWith([legacy(fighters, 'fortress cancelled one hit', Math.min(4, fighters))]);
    const side = battle(game).attacker;
    assert.equal(side.cancelledHits?.before, 5);
    assert.equal(side.nominalHits, 4);
    assert.equal(side.hits, fighters);
    assert.deepEqual(battleCues(game, 0)[0].combat.attacker, side);
  });

test('China/Celts 29 value: Elephant + disembark WarShips cancel 5 to 3, paid Tactician makes 2', () => {
  const roll: LoggedAction = {
    log: [
      'Combat round 1',
      'Player1: Combat: Attacking with 1 infantry and 1 cavalry and 1 elephant and Qin Shi Huang',
      'Player2: Combat: Defending with 2 infantry and Vercingetorix and 2 settlers',
      'Player1: Combat: Roll 4 (elephant, -1 hits, no combat value) for combined combat value of 0 and gets 0 hits against defending units',
      'Player1: Combat: Combat modifiers: WarShips ignore the first hit in the first round of combat',
      'Player2: Combat: Roll 6 for combined combat value of 29 and gets 3 hits against attacking units',
    ],
  };
  const pay: LoggedAction = { log: ['Player1: Tactician: Pay 2 culture tokens, Ignore the first hit'] };
  const game = gameWith([roll, pay]);
  const before = battleCues(game, 0, 1)[0].combat.defender;
  assert.equal(before.cancelledHits?.before, 5);
  assert.equal(before.nominalHits, 3);
  assert.equal(before.hits, 3);
  const cue = battleCues(game, 1, 2)[0];
  assert.equal(cue.roll, false);
  assert.equal(cue.combat.defender.cancelledHits?.before, 5);
  assert.equal(cue.combat.defender.nominalHits, 2);
  assert.equal(cue.combat.defender.hits, 2);
  assert.deepEqual(cue.combat, battle(game));
  assert.equal(roll.log!.at(-1)?.includes('gets 3 hits'), true, 'history is not rewritten');
});

test('overkill without cancellation and zero hits never acquire a false strike-through', () => {
  for (const [value, hits, nominal] of [
    [29, 3, 5],
    [0, 0, 0],
  ]) {
    const side = battle(gameWith([legacy(3, '', hits, value)])).attacker;
    assert.equal(side.nominalHits, nominal);
    assert.equal(side.hits, hits);
    assert.equal(side.cancelledHits, undefined);
  }
});

test('explicit multi-hit cancellation quantities, unknown legacy modifiers, and failed payments', () => {
  const side = battle(gameWith([legacy(4, 'Shield cancels 2 hits', 3)])).attacker;
  assert.equal(side.nominalHits, 3);
  assert.equal(side.cancelledHits?.before, 5);
  const unknown = battle(gameWith([legacy(4, 'Unknown cancels hits', 2)])).attacker;
  assert.equal(unknown.nominalHits, undefined);
  assert.equal(unknown.cancelledHits, undefined);
  for (const note of [
    "Won't reduce the hits, no payment made.",
    'Not enough resources, no payment made.',
    'Pay 2 culture tokens',
  ]) {
    const side = battle(gameWith([legacy(4, '', 4), { log: [`Player2: Tactician: ${note}`] }])).attacker;
    assert.equal(side.nominalHits, 5);
    assert.equal(side.hits, 4);
  }
});

test('structured hit reports retain uncancellable extras and later authoritative totals without double subtraction', () => {
  const initial: LoggedAction = {
    items: [
      {
        player: 0,
        CombatRound: {
          round: 1,
          attackers: { infantry: 3 },
          defenders: { infantry: 4 },
          defending_player: 1,
        },
      },
      {
        player: 0,
        CombatRoll: { rolls: [{ value: 6, unit_type: 'Infantry', bonus: false }], combat_value: 29, hits: 4 },
      },
      { player: 0, CombatHitReport: { combat_value: 29, hit_cancels: 2, extra_hits: 1, casualties: 4 } },
    ],
  };
  const payment: LoggedAction = {
    items: [{ player: 1, origin: { SpecialAdvance: 'Fireworks' }, Text: 'Ignore the first hit' }],
  };
  const final: LoggedAction = {
    items: [
      { player: 0, CombatHitReport: { combat_value: 29, hit_cancels: 3, extra_hits: 1, casualties: 3 } },
    ],
  };
  const game = gameWith([initial, payment, final]);
  assert.equal(battleCues(game, 0, 1)[0].combat.attacker.cancelledHits?.before, 6);
  assert.equal(battleCues(game, 0, 1)[0].combat.attacker.nominalHits, 4);
  assert.equal(battleCues(game, 1, 2)[0].combat.attacker.nominalHits, 3);
  const side = battle(game).attacker;
  assert.equal(side.nominalHits, 3);
  assert.equal(side.hits, 3);
  assert.equal(side.cancelledHits?.before, 6);
  // No cancellations can remove the extra hits, even with zero combat value.
  final.items![0].CombatHitReport = { combat_value: 0, hit_cancels: 7, extra_hits: 2, casualties: 2 };
  const extra = battle(game).attacker;
  assert.equal(extra.nominalHits, 2);
  assert.equal(extra.cancelledHits, undefined);
});

test('Fortress, WarShips and Elephant combine explicitly; ambiguous prose never becomes arithmetic', () => {
  const roll = legacy(
    4,
    'fortress cancelled one hit, WarShips ignore the first hit in the first round of combat',
    2,
  );
  roll.log!.push(
    'Player2: Combat: Roll 4 (elephant, -1 hits, no combat value) for combined combat value of 0 and gets 0 hits against attacking units',
  );
  const side = battle(gameWith([roll])).attacker;
  assert.equal(side.cancelledHits?.before, 5);
  assert.equal(side.nominalHits, 2);
  assert.equal(side.hits, 2);
  for (const reason of ['Unknown cancels hits', 'Shield cancels 1 hit and another shield cancels 1 hit']) {
    const unknown = battle(gameWith([legacy(3, reason, 3)])).attacker;
    assert.equal(unknown.nominalHits, undefined);
    assert.equal(unknown.cancelledHits, undefined);
    assert.equal(unknown.hits, 3);
  }
});

test('real engine Fireworks reports survive saved JSON and public filtering for paid/declined/unaffordable responses', async () => {
  const { createRequire } = await import('node:module');
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const npcs = JSON.parse(await engine.init(2, [], {}, 'hit-reports', {})).players.slice(2);
  const items = (raw: string) => JSON.parse(raw).log.flatMap((a: any) => a.rounds.flatMap((r: any) =>
    r.turns.flatMap((t: any) => (t.actions ?? []).flatMap((a: any) => a.items ?? []))));
  const reports = (raw: string) => items(raw).filter((i: any) => i.player === 1 && i.CombatHitReport).map((i: any) => i.CombatHitReport);
  for (const outcome of ['paid', 'declined', 'unaffordable']) {
    const fixture = JSON.parse(readFileSync(new URL('../../server/tests/test_games/civilizations/china/fireworks.json', import.meta.url), 'utf8'));
    for (const npc of npcs) if (!fixture.players.some((p: any) => p.civilization === npc.civilization)) fixture.players.push(structuredClone(npc));
    if (outcome === 'unaffordable') fixture.players[0].resources = {};
    let raw = JSON.stringify(fixture);
    const query = JSON.parse(engine.webQuery(raw, 0, JSON.stringify({ kind: 'movement', units: [0] })));
    raw = engine.tryMove(raw, JSON.stringify(query.destinations.find((d: any) => d.position === 'C1' && d.attack).action), 0);
    const rolls = items(raw).filter((i: any) => i.CombatRoll);
    if (outcome !== 'unaffordable') {
      assert.deepEqual(reports(raw), [{combat_value: 6, hit_cancels: 0, extra_hits: 0, casualties: 1}]);
      raw = engine.tryMove(raw, JSON.stringify({Response: {Payment: [outcome === 'paid' ? {gold: 2} : {}]}}), 0);
    }
    assert.deepEqual(reports(raw).slice(0, 2), [
      {combat_value: 6, hit_cancels: 0, extra_hits: 0, casualties: 1},
      {combat_value: 6, hit_cancels: outcome === 'paid' ? 1 : 0, extra_hits: 0, casualties: outcome === 'paid' ? 0 : 1},
    ]);
    assert.deepEqual(items(raw).filter((i: any) => i.CombatRoll).slice(0, rolls.length), rolls);
    for (const seat of [undefined, 0, 1]) {
      const visible = engine.stripSecret(raw, seat);
      assert.deepEqual(reports(visible), reports(raw));
      const parsed = JSON.parse(visible);
      assert.ok(parsed.players.every((p: any, index: number) => index === seat || (p.action_cards ?? []).every((c: number) => c === 0)));
      const combat = battle(parsed);
      assert.equal(combat.defender.nominalHits, outcome === 'paid' ? 0 : 1);
      assert.equal(combat.defender.hits, outcome === 'paid' ? 0 : 1);
    }
  }
});
