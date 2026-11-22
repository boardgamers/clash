import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { battleCues, battleCursor } from './battle-playback.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'horse-master', {})).players.slice(2);
const items = (g: any) =>
  g.log.flatMap((a: any) =>
    a.rounds.flatMap((r: any) =>
      r.turns.flatMap((t: any) => (t.actions ?? []).flatMap((a: any) => a.items ?? [])),
    ),
  );
function respond(raw: string, response: object) {
  return engine.tryMove(raw, JSON.stringify({ Response: response }), engine.currentPlayer(raw));
}
function battle(leader = 'Attila', cavalry = true) {
  const g = JSON.parse(
    readFileSync(new URL('../../server/tests/test_games/combat/retreat.json', import.meta.url), 'utf8'),
  );
  g.players.push(...structuredClone(npcs));
  for (let p = 0; p < 2; p++) {
    g.players[p].civilization = p === 0 ? 'India' : 'Huns';
    g.players[p].advances = ['Farming', 'Mining', 'Tactics', ...(p === 1 ? ['SteelWeapons'] : [])];
    g.players[p].units = [
      { id: 0, position: p === 0 ? 'C2' : 'C1', unit_type: p === 0 ? 'Infantry' : { Leader: leader } },
      { id: 1, position: p === 0 ? 'C2' : 'C1', unit_type: p === 0 || !cavalry ? 'Infantry' : 'Cavalry' },
    ];
    g.players[p].next_unit_id = 2;
    g.players[p].action_cards = [p === 0 ? 31 : 9];
  }
  g.dice_roll_outcomes = [10, 2, 9, 3, 9];
  let raw = engine.tryMove(
    JSON.stringify(g),
    JSON.stringify({ Movement: { Move: { units: [0, 1], destination: 'C1', payment: {} } } }),
    0,
  );
  raw = respond(raw, { SelectHandCards: [{ ActionCard: 31 }] });
  raw = respond(raw, { Payment: [{ ore: 1 }] });
  return respond(raw, { SelectHandCards: [{ ActionCard: 9 }] });
}
for (const accept of [true, false])
  test(`Horse Master defender fixed dice: accept=${accept}`, () => {
    const beforeRaw = battle();
    const before = JSON.parse(beforeRaw);
    assert.match(before.events.at(-1).handler.request.BoolRequest, /Horse Master/);
    assert.equal(before.players[0].units.length, 2);
    assert.equal(before.players[1].units.length, 2);
    const rolled = items(before).filter((i: any) => i.CombatRoll);
    assert.deepEqual(
      rolled.map((i: any) => i.CombatRoll.combat_value),
      [12, 12],
    );
    assert.deepEqual(
      rolled[0].CombatRoll.rolls.map((d: any) => [d.value, d.unit_type, d.bonus]),
      [
        [5, 'Infantry', false],
        [2, 'Elephant', false],
        [5, 'Infantry', false],
      ],
    );
    assert.deepEqual(
      rolled[1].CombatRoll.rolls.map((d: any) => [d.value, d.unit_type, d.bonus]),
      [
        [2, 'Cavalry', true],
        [6, 'Infantry', false],
      ],
    );
    let raw = respond(beforeRaw, { Bool: accept });
    let after = JSON.parse(raw);
    assert.deepEqual(
      items(after).slice(0, items(before).length),
      items(before),
      'never rewrite original dice or reports',
    );
    const final = [0, 1].map(
      (p) =>
        items(after)
          .filter((i: any) => i.player === p && i.CombatHitReport)
          .at(-1).CombatHitReport,
    );
    // Both final reports are emitted before either side removes casualties.
    assert.deepEqual(final, [
      { combat_value: 12, hit_cancels: accept ? 1 : 0, extra_hits: 0, casualties: accept ? 1 : 2 },
      { combat_value: accept ? 8 : 12, hit_cancels: 0, extra_hits: 0, casualties: accept ? 1 : 2 },
    ]);
    const cue = battleCues(after, battleCursor(before)).at(-1)!;
    assert.equal(cue.roll, false, 'conversion is not another dice animation');
    assert.equal(cue.combat.defender.value, accept ? 8 : 12);
    assert.equal(cue.combat.attacker.hits, accept ? 1 : 2);
    assert.deepEqual(cue.combat.defender.dice, battleCues(before, 0).at(-1)!.combat.defender.dice);
    if (accept) {
      assert.equal(after.events.at(-1).handler.request.SelectUnits.needed.start, 1);
      raw = respond(raw, { SelectUnits: [1] });
      after = JSON.parse(raw);
      assert.equal(after.players[0].units.length, 1);
      assert.equal(after.players[1].units.length, 1);
      after.dice_roll_outcomes = [3, 3, 3, 3];
      after = JSON.parse(respond(JSON.stringify(after), { Bool: false }));
      assert.equal(after.events.at(-1).event_type.CombatRoundEnd.combat.stats.round, 2);
      assert.equal(after.events.at(-1).handler.request.BoolRequest, 'Do you want to retreat?');
      assert.equal(after.players[0].units.length, 1);
      assert.equal(after.players[1].units.length, 1);
    } else {
      assert.equal(after.players[0].units?.length ?? 0, 0);
      assert.equal(after.players[1].units?.length ?? 0, 0);
    }
  });
for (const [leader, cavalry] of [
  ['Bleda', true],
  ['Attila', false],
] as const)
  test(`Horse Master requires Attila and an activated cavalry: ${leader}/${cavalry}`, () => {
    const g = JSON.parse(battle(leader, cavalry));
    assert.ok(!g.events?.some((e: any) => e.handler?.request.BoolRequest?.startsWith('Horse Master')));
  });

test('published engine 0.4.165 pending Horse Master preserves its original outcome', () => {
  const frozen = JSON.parse(
    readFileSync(
      new URL('../../server/tests/fixtures/legacy_horse_master_165.json', import.meta.url),
      'utf8',
    ),
  );
  assert.equal(frozen.engine, '0.4.165');
  assert.equal(frozen.pending.players[0].units?.length ?? 0, 0);
  const resumed = JSON.parse(respond(JSON.stringify(frozen.pending), { Bool: true }));
  for (const key of ['players', 'events', 'state', 'rng', 'dice_roll_outcomes', 'log', 'log_index']) {
    assert.deepEqual(resumed[key], frozen.accepted[key], `old pending battle preserves ${key}`);
  }
});

test('published 0.4.165 legal history replays full and partial cursors with a new-battle boundary', () => {
  const frozen = JSON.parse(
    readFileSync(
      new URL('../../server/tests/fixtures/legacy_horse_master_165.json', import.meta.url),
      'utf8',
    ),
  );
  for (const { to, state } of frozen.cursors) {
    const replay = JSON.parse(engine.replay(JSON.stringify(frozen.history), { to }));
    for (const key of ['players', 'events', 'state', 'rng', 'log_index']) {
      assert.deepEqual(replay[key], state[key], `old legal cursor ${to} preserves ${key}`);
    }
    assert.equal(replay.finalized_combat_from, to + 1);
  }
});
