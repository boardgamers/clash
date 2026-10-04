import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { battleCues, historyThrough } from './battle-playback.ts';
import type { Game } from './types.ts';
const fixture = (name: string): Game =>
  JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8'),
  );

test('battle playback keeps recorded naval dice, hit cancellations, leaders and battlefield locations', () => {
  const [naval] = battleCues(fixture('ship_combat.outcome'), 0);
  assert.equal(naval.location?.defender.position, 'D2');
  assert.deepEqual(
    naval.combat.attacker.dice?.map((d) => d.value),
    [6, 6],
  );
  assert.deepEqual(naval.losses, [{ civilization: 'Greece', label: 'ship', value: '−1' }]);
  const [land] = battleCues(fixture('direct_capture_city_metallurgy.outcome'), 0);
  assert.equal(land.combat.defender.hits, 0);
  assert.equal(land.combat.defender.cancelledHits?.before, 1);
  assert.ok(land.losses.some((l) => l.label === 'Caesar'));
  assert.deepEqual(Object.keys(land.location!.attacker).sort(), ['player', 'position']);
});

test('replay cuts off later rounds, casualties and results before parsing the journal', () => {
  const game = fixture('retreat_no.outcome1');
  const first = battleCues(game, 0, 1);
  assert.equal(first.length, 1);
  assert.equal(first[0].combat.round, 1);
  assert.equal(first[0].combat.attacker.value, 1);
  assert.equal(first[0].combat.result, undefined);
  assert.deepEqual(first[0].losses, []);
  assert.equal(first[0].location, null, 'do not borrow a future final combat record');
  const second = battleCues(game, 1, 2);
  assert.equal(second.length, 1);
  assert.equal(second[0].combat.round, 2);
  assert.equal(second[0].combat.result, 'Greece wins');
  assert.equal(second[0].location?.round, 2);
  assert.equal(second[0].losses.length, 2);
  assert.equal(battleCues({ ...game, log_index: 1 }, 1, 2).length, 0, 'undone actions never animate');
  assert.equal(battleCues(game, 2, 1).length, 0, 'reverse intervals do not invent a battle');
});

test('multiple completed rounds queue in order, and unrelated later actions do not replay a battle', () => {
  const game = fixture('retreat_no.outcome1');
  assert.deepEqual(
    battleCues(game, 0).map((c) => c.combat.round),
    [1, 2],
  );
  const actions = game.log![0].rounds[0].turns[0].actions!;
  actions.push({ action: { Playing: 'EndTurn' }, log: ['Player1: Turn: Ends turn'] });
  game.log_index = actions.length;
  assert.deepEqual(battleCues(game, 2), []);
  assert.equal(historyThrough(game, 1).log![0].rounds[0].turns[0].actions!.length, 1);
});

test('recorded pending locations and legacy prose work without live event data', () => {
  const game = fixture('retreat_no.outcome1');
  game.events = [];
  game.board_history = {
    id: 'test',
    frames: [
      {
        cursor: 1,
        combat: {
          round: 1,
          attacker: { player: 0, position: 'C2' },
          defender: { player: 1, position: 'C1' },
        },
      } as any,
    ],
  };
  assert.equal(battleCues(game, 0, 1)[0].location?.defender.position, 'C1');
  game.log![0].rounds[0].turns[0].actions = [
    {
      log: [
        'Combat round 1',
        'Player1: Combat: Attacking with 1 infantry, Roll 6 (infantry, +1 combat value) for combined combat value of 7 and gets 1 hits against defending units',
        'Player2: Combat: Defending with 1 infantry, Roll 1 (leader, no bonus) for combined combat value of 1 and gets 0 hits against attacking units',
        'Attacker wins',
      ],
    },
  ];
  game.log_index = 1;
  assert.equal(battleCues(game, 0)[0].combat.attacker.hits, 1);
});

test('rolls arriving after a tactics prompt stay assigned to their public battle participants', () => {
  const game = fixture('retreat_no.outcome1');
  game.log![0].rounds[0].turns[0].actions = [
    {
      items: [
        {
          player: 0,
          origin: { Ability: 'Combat' },
          CombatRound: {
            round: 1,
            attackers: { infantry: 1 },
            defenders: { infantry: 2 },
            defending_player: 1,
          },
        },
      ],
    },
    {
      items: [
        {
          player: 0,
          origin: { Ability: 'Combat' },
          CombatRoll: { rolls: [{ value: 6, unit_type: 'Infantry', bonus: true }], combat_value: 7, hits: 1 },
        },
        {
          player: 1,
          origin: { Ability: 'Combat' },
          CombatRoll: { rolls: [{ value: 1, unit_type: 'Leader', bonus: false }], combat_value: 1, hits: 0 },
        },
      ],
    },
  ];
  game.log_index = 2;
  const [cue] = battleCues(game, 1, 2);
  assert.equal(cue.combat.attacker.value, 7);
  assert.equal(cue.combat.defender.value, 1);
  assert.equal(cue.combat.attacker.hits, 1);
  assert.equal(cue.combat.defender.hits, 0);
});
