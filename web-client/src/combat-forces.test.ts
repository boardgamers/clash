import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { combatForces } from './combat-forces.ts';
import type { Game } from './types.ts';

function battle(): Game {
  return {
    state: null,
    current_player_index: 1,
    actions_left: 0,
    age: 1,
    round: 1,
    log_index: 0,
    events: [
      {
        event_type: {
          CombatRoundStart: {
            combat: {
              attackers: [1, 2, 3],
              stats: {
                round: 1,
                attacker: { player: 0, position: 'C2' },
                defender: { player: 1, position: 'C1' },
              },
            },
          },
        },
      },
    ],
    players: [
      {
        id: 0,
        civilization: 'Rome',
        units: [1, 2, 3, 4].map((id) => ({
          id,
          position: 'C2',
          unit_type: id === 3 ? 'Cavalry' : 'Infantry',
        })),
      },
      {
        id: 1,
        civilization: 'Greece',
        units: [
          { id: 1, position: 'C1', unit_type: 'Infantry' },
          { id: 2, position: 'C1', unit_type: 'Settler' },
        ],
      },
    ],
    map: { tiles: [['C1', 'Fertile']] },
  } as Game;
}

test('defender tactics shows only three incoming participants, not four in source hex', () => {
  const game = battle();
  game.current_player_index = 1;
  game.events!.push({ event_type: { Ability: { name: 'Tactics' } } });
  const before = JSON.stringify(game);
  const forces = combatForces(game)!;
  assert.equal(forces.attacker!.count, 3);
  assert.deepEqual(
    forces.attacker!.units.map((u) => [u.label, u.count]),
    [
      ['Infantry', 2],
      ['Cavalry', 1],
    ],
  );
  assert.equal(forces.defender!.count, 1);
  assert.equal(JSON.stringify(game), before);
});

test('actual pending combat fixture resolves fighters using surviving engaged IDs', () => {
  const game: Game = JSON.parse(
    readFileSync(
      new URL(
        '../../server/tests/test_games/combat/remove_casualties_attacker.outcome.json',
        import.meta.url,
      ),
      'utf8',
    ),
  );
  const forces = combatForces(game)!;
  assert.ok(forces.attacker && forces.defender);
  assert.ok(forces.attacker.count > 0);
});

test('removed casualties are no longer counted and ended battles disappear', () => {
  const game = battle();
  const event = game.events![0].event_type as any;
  event.CombatRoundStart.combat.attackers = [1, 3];
  event.CombatRoundStart.combat.stats.round = 2;
  game.players[0].units = game.players[0].units!.filter((u) => u.id !== 2);
  assert.equal(combatForces(game)!.attacker!.count, 2);
  assert.equal(combatForces(game)!.round, 2);
  game.events!.push({ event_type: { CombatEnd: {} } });
  assert.equal(combatForces(game), null);
});

test('sea battle counts ships, not passengers, settlers, or extra dice', () => {
  const game = battle();
  game.map.tiles = [['C1', 'Water']];
  game.players[0].units![0].unit_type = 'Ship';
  game.players[0].units![1].carrier_id = 1;
  game.players[1].units![0].unit_type = 'Ship';
  assert.equal(combatForces(game)!.attacker!.count, 1);
  assert.equal(combatForces(game)!.defender!.count, 1);
});

test('missing visible IDs or units remain unknown rather than using source stack/history', () => {
  const game = battle();
  game.players[0].units = game.players[0].units!.filter((u) => u.id !== 3);
  assert.equal(combatForces(game)!.attacker, null);
  delete (game.events![0].event_type as any).CombatRoundStart.combat.attackers;
  assert.equal(combatForces(game)!.attacker, null);
  delete game.players[1].units;
  assert.equal(combatForces(game)!.defender, null);
  assert.equal(combatForces(null), null);
});

test('live force summary is mounted before decision options and suppressed in playback', () => {
  const source = readFileSync(new URL('./DecisionPanel.svelte', import.meta.url), 'utf8');
  assert.match(source, /\{#if !\$session.playback\}<CombatForces game=\{\$session.game\}/);
  assert.ok(source.indexOf('<CombatForces') < source.indexOf('<TacticsOption'));
});

test('pre-roll CombatStart is supported and an empty fortress defense is zero units', () => {
  const game = battle();
  const combat = (game.events![0].event_type as any).CombatRoundStart.combat;
  game.events = [{ event_type: { CombatStart: combat } }];
  game.players[1].units = [];
  game.players[1].cities = [{ position: 'C1', mood_state: 'Happy', city_pieces: { fortress: 1 } }];
  assert.equal(combatForces(game)!.attacker!.count, 3);
  assert.equal(combatForces(game)!.defender!.count, 0);
});
