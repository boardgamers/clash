import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)(process.env.COMBAT_ENGINE ?? '../.engine/server.js');
function battle() {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/combat/direct_capture_city_fortress.json', import.meta.url),
      'utf8',
    ),
  );
  g.players[0].advances = ['Farming', 'Mining', 'Tactics', 'SteelWeapons'];
  g.players[1].advances = ['Farming', 'Mining', 'Tactics'];
  g.players[0].units = [0, 1, 2, 3].map((id) => ({ id, position: 'C2', unit_type: 'Infantry' }));
  g.players[1].units = [0, 1].map((id) => ({ id, position: 'C1', unit_type: 'Infantry' }));
  g.players[1].cities = [{ position: 'C1', mood_state: 'Happy', city_pieces: { fortress: 1 } }];
  g.events = [
    {
      event_type: {
        CombatRoundStart: {
          phase: 'Default',
          combat: {
            attackers: [0, 1, 2],
            retreat: 'CanRetreat',
            modifiers: [],
            stats: {
              round: 1,
              battleground: 'CityWithFortress',
              attacker: { player: 0, position: 'C2', present: { infantry: 3 } },
              defender: { player: 1, position: 'C1', present: { infantry: 2 } },
            },
          },
          attacker_strength: {},
          defender_strength: {},
        },
      },
      players_used: [0],
      player: 1,
      last_priority_used: 0,
      handler: {
        priority: 0,
        request: {
          SelectHandCards: { choices: [], needed: { start: 0, end: 1 }, description: 'Play Tactics Card' },
        },
        origin: { Advance: 'Tactics' },
      },
    },
  ];
  return g;
}
const round = (g: any) => g.events[0].event_type.CombatRoundStart;
function summary(g: any, seat = 1) {
  return JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), seat), seat)).combatSummary;
}
test('engine public summary counts the actual three attackers and first-round fortress die/protection', () => {
  const g = battle();
  const before = JSON.stringify(g);
  const s = summary(g);
  assert.equal(s.attacker.dice, 3);
  assert.equal(s.defender.dice, 3);
  assert.equal(s.defender.extraDice, 1);
  assert.equal(s.defender.hitCancels, 1);
  assert.equal(JSON.stringify(g), before);
  round(g).combat.stats.round = 2;
  assert.equal(summary(g).defender.dice, 2);
  assert.equal(summary(g).defender.hitCancels, 0);
});
test('unpaid Steel Weapons is conditional, paid +2/+1 is value not extra dice', () => {
  const g = battle();
  assert.equal(summary(g).attacker.valueBonus, 0);
  assert.match(summary(g).attacker.inactive[0], /not activated: \+2/);
  round(g).combat.modifiers.push('SteelWeaponsAttacker');
  assert.equal(summary(g).attacker.valueBonus, 2);
  assert.equal(summary(g).attacker.dice, 3);
  assert.deepEqual(summary(g).attacker.inactive, []);
  g.players[1].advances.push('SteelWeapons');
  assert.equal(summary(g).attacker.valueBonus, 1);
  assert.equal(summary(g).defender.valueBonus, 0);
  assert.match(summary(g).defender.inactive[0], /not activated: \+1/);
  round(g).combat.modifiers.push('SteelWeaponsDefender');
  assert.equal(summary(g).defender.valueBonus, 1);
});
test('Siegecraft cancellations use the same fortress rules as resolution', () => {
  const g = battle();
  round(g).combat.modifiers.push('CancelFortressExtraDie');
  assert.equal(summary(g).defender.dice, 2);
  assert.equal(summary(g).defender.hitCancels, 1);
  round(g).combat.modifiers.push('CancelFortressIgnoreHit');
  assert.equal(summary(g).defender.hitCancels, 0);
});
test('hidden tactics, hands and random seeds cannot change public bonus projection', () => {
  const g = battle();
  const base = summary(g);
  round(g).attacker_strength.tactics_card = 6;
  g.players[0].action_cards = [7, 8];
  g.seed = '999';
  g.dice_roll_outcomes = [0, 0, 0];
  assert.deepEqual(summary(g), base);
  assert.deepEqual(summary(g, 0), base);
  round(g).attacker_strength.tactics_card = 9;
  assert.deepEqual(summary(g), base);
});
test('naval combat excludes Steel Weapons even with stale paid modifiers', () => {
  const g = battle();
  g.map.tiles = g.map.tiles.map(([p, t]: any) => [p, p === 'C1' ? 'Water' : t]);
  for (const p of g.players.slice(0, 2)) for (const u of p.units) u.unit_type = 'Ship';
  g.players[1].cities = [];
  round(g).combat.modifiers.push('SteelWeaponsAttacker');
  assert.equal(summary(g).attacker.valueBonus, 0);
  assert.equal(summary(g).defender.extraDice, 0);
});

test('Great Wall penalty and Great Warlord value use public engine rules', () => {
  const g = battle();
  g.players[1].cities[0].city_pieces.wonders = ['GreatWall'];
  assert.equal(summary(g).attacker.valueBonus, -2);
  assert.equal(summary(g).defender.valueBonus, 0);
  round(g).combat.modifiers.push('GreatWarlord');
  assert.equal(summary(g).attacker.valueBonus, 0);
  assert.match(summary(g).attacker.effects.join(' '), /Great Warlord/);
  round(g).combat.stats.round = 2;
  assert.equal(summary(g).attacker.valueBonus, 2);
});

test('empty fortress has one die, not a phantom unit; ended rounds stop projecting', () => {
  const g = battle();
  g.players[1].units = [];
  assert.equal(summary(g).defender.dice, 1);
  g.events = [];
  assert.equal(summary(g), null);
});

test('repeated cached seat and spectator projections do not mutate public state or subsequent views', () => {
  const g = battle();
  const raw = JSON.stringify(g);
  const publicRaw = engine.stripSecret(raw, 1);
  const first = engine.webView(publicRaw, 1);
  for (let i = 0; i < 4; i++) {
    const spectator = engine.stripSecret(raw);
    assert.deepEqual(JSON.parse(engine.webView(spectator)).combatSummary, summary(g));
    summary(g, 0);
    assert.equal(engine.webView(publicRaw, 1), first);
    assert.equal(engine.stripSecret(raw, 1), publicRaw);
  }
  assert.equal(JSON.stringify(g), raw);
});
