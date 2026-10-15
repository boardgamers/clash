import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { journal } from './journal.ts';
import { combatJournal } from './combat-journal.ts';
import { steelWeaponsBenefit } from './active-combat.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'card-rules', {})).players.slice(2);
function fixture(name: string) {
  const g = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  g.players = [...g.players.slice(0, 2), ...structuredClone(npcs)];
  return g;
}
const move = (
  g: any,
  action: unknown,
  seat = engine.currentPlayer(typeof g === 'string' ? g : JSON.stringify(g)),
) => JSON.parse(engine.tryMove(typeof g === 'string' ? g : JSON.stringify(g), JSON.stringify(action), seat));
const view = (g: any, seat = engine.currentPlayer(JSON.stringify(g))) =>
  JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), seat), seat));
const attack = (g: any, units = [0, 1, 2, 3]) =>
  move(g, { Movement: { Move: { units, destination: 'C1', payment: {} } } }, 0);
const offersConqueror = (g: any) =>
  g.events?.some((e: any) => e.event_type?.SelectObjectives?.shown_objective === 'Conqueror') ?? false;

for (const defense of ['army', 'fortress', 'empty', 'settler', 'barbarian']) {
  test(`Conqueror checks the captured city's owner and defenders: ${defense}`, () => {
    const g = fixture('objective_cards/instant/conqueror');
    g.players[0].objective_cards = [8];
    if (defense !== 'army') g.players[1].units = [];
    if (defense !== 'fortress') g.players[1].cities[0].city_pieces = {};
    if (defense === 'settler') g.players[1].units = [{ id: 0, position: 'C1', unit_type: 'Settler' }];
    if (defense === 'barbarian') {
      g.players[2].cities = g.players[1].cities;
      g.players[2].units = [{ id: 0, position: 'C1', unit_type: 'Infantry' }];
      g.players[1].cities = [];
    }
    const after = attack(g);
    assert.ok(
      after.players[0].cities.some((c: any) => c.position === 'C1'),
      'The attack captures the city',
    );
    assert.equal(offersConqueror(after), defense === 'army' || defense === 'fortress');
  });
}

test('defending your own city does not complete Conqueror', () => {
  const g = fixture('objective_cards/instant/conqueror');
  g.players[0].objective_cards = [];
  g.players[1].objective_cards = [8];
  g.players[0].units = g.players[0].units.slice(0, 1);
  g.dice_roll_outcomes = [0, 11, 11, 11, 11, 0, 11, 11, 11, 11];
  const after = attack(g, [0]);
  assert.equal((after.players[0].units ?? []).length, 0);
  assert.ok(after.players[1].cities.some((c: any) => c.position === 'C1'));
  assert.equal(offersConqueror(after), false);
});

test('Metallurgy activates Steel Weapons against barbarians for free and explains the bonus', () => {
  const g = fixture('combat/direct_capture_city_metallurgy');
  g.players[2].cities = g.players[1].cities;
  g.players[2].units = [{ id: 0, position: 'C1', unit_type: 'Infantry' }];
  g.players[1].cities = [];
  g.players[1].units = [];
  const ore = g.players[0].resources.ore;
  const after = attack(g);
  assert.equal(after.players[0].resources.ore, ore);
  assert.ok(!after.events?.some((e: any) => e.handler?.origin?.Advance === 'SteelWeapons'));
  const combat = combatJournal(journal(after)).find((entry) => entry.combat)?.combat;
  assert.ok(
    combat?.attacker.modifiers.includes('steel weapons added 2 combat value (Metallurgy: no ore cost)'),
  );
});

for (const opponentHasSteel of [false, true]) {
  test(`Steel Weapons requests and spends ore when ${opponentHasSteel ? 'both players have it, even with Metallurgy' : 'Metallurgy is absent'}`, () => {
    const g = fixture('combat/direct_capture_city_metallurgy');
    if (opponentHasSteel) g.players[1].advances.push('SteelWeapons');
    else g.players[0].advances = g.players[0].advances.filter((a: string) => a !== 'Metallurgy');
    const before = g.players[0].resources.ore;
    let after = attack(g);
    assert.equal(
      steelWeaponsBenefit(after, view(after), 0),
      opponentHasSteel
        ? '+1 combat value each round. Enemy has Steel Weapons.'
        : '+2 combat value each round.',
    );
    const prompt = after.events.at(-1).handler;
    assert.equal(prompt.origin.Advance, 'SteelWeapons');
    assert.deepEqual(prompt.request.Payment[0].cost.default, { ore: 1 });
    after = move(after, { Response: { Payment: [{ ore: 1 }] } }, 0);
    assert.equal(after.players[0].resources.ore, before - 1);
  });
}

test('Steel Weapons prompt accounts for borrowed enemy advances and either combat role', () => {
  const g = fixture('combat/direct_capture_city_metallurgy');
  g.players[1].great_library_advance = 'SteelWeapons';
  const after = attack(g);
  const publicView = view(after);
  assert.equal(
    steelWeaponsBenefit(after, publicView, 0),
    '+1 combat value each round. Enemy has Steel Weapons.',
  );
  assert.equal(
    steelWeaponsBenefit(after, publicView, 1),
    '+1 combat value each round. Enemy has Steel Weapons.',
  );
  assert.equal(steelWeaponsBenefit(after, publicView, 2), null);
  assert.equal(steelWeaponsBenefit({ ...after, events: [] }, publicView, 0), null);
});

test('leader-only recruitment is offered with a real cost, and unavailable cities explain why', async () => {
  let g: any = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'leader-availability', {});
  g = move(g, { ChooseCivilization: 'China' });
  g = move(g, { ChooseCivilization: 'Greece' });
  const seat = engine.currentPlayer(JSON.stringify(g));
  const p = g.players[seat];
  p.cities[0].mood_state = 'Happy';
  p.resources = { mood_tokens: 1, culture_tokens: 1 };
  const city = view(g).cityActions[0];
  assert.ok(city.recruits.every((r: any) => r.reason));
  assert.ok(city.buildings.every((b: any) => !b.choices.length));
  assert.equal(city.happiness.length, 0);
  const leader = city.leaders.find((l: any) => l.reason === null);
  assert.ok(leader, 'A leader remains recruitable when all other city actions are unavailable');
  assert.deepEqual(leader.payment, { mood_tokens: 1, culture_tokens: 1 });
  const q = JSON.parse(
    engine.webQuery(
      engine.stripSecret(JSON.stringify(g), seat),
      seat,
      JSON.stringify({ kind: 'recruit', city: city.position, units: { leader: leader.id }, replaced: [] }),
    ),
  );
  const after = move(g, q.action);
  assert.ok(after.players[seat].units.some((u: any) => u.unit_type?.Leader === leader.id));
  assert.equal(after.players[seat].resources?.mood_tokens ?? 0, 0);
  p.resources = {};
  assert.ok(view(g).cityActions[0].leaders.every((l: any) => l.reason));
  assert.ok(view(g, 1 - seat).cityActions[0].leaders.every((l: any) => !!l.reason));
});

for (const metallurgy of [false, true]) {
  for (const pirates of [false, true]) {
    test(`Steel Weapons never activates in naval combat (Metallurgy=${metallurgy}, pirates=${pirates})`, () => {
      const g = fixture('combat/ship_combat');
      for (const p of g.players.slice(0, 2)) {
        p.advances.push('SteelWeapons', ...(metallurgy ? ['Metallurgy'] : []));
        p.action_cards = []; p.objective_cards = [];
        p.resources.ore = 3;
      }
      if (pirates) {
        g.players[3].units = g.players[1].units;
        g.players[1].units = [];
      }
      const after = move(g, { Movement: { Move: { units: [7, 8], destination: 'D2' } } }, 0);
      assert.ok(!after.events?.some((e: any) => e.handler?.origin?.Advance === 'SteelWeapons'));
      assert.equal(after.players[0].resources.ore, 3);
      assert.equal(after.players[1].resources.ore, 3);
      const battles = combatJournal(journal(after)).filter((entry) => entry.combat);
      assert.ok(battles.length, 'the ships actually fought');
      for (const entry of battles) {
        assert.ok(![...entry.combat!.attacker.modifiers, ...entry.combat!.defender.modifiers].some((m: string) => /steel weapons/i.test(m)));
      }
    });
  }
}

test('Steel Weapons remains available when troops disembark into a land battle', () => {
  const g = fixture('combat/direct_capture_city_metallurgy');
  g.players[0].advances = g.players[0].advances.filter((a: string) => a !== 'Metallurgy');
  g.map.tiles.find((t: any) => t[0] === 'C2')[1] = 'Water';
  g.players[0].cities = g.players[0].cities.filter((c: any) => c.position !== 'C2');
  g.players[0].units = [{ id: 99, unit_type: 'Ship', position: 'C2', carried_units: [{ id: 0, unit_type: 'Infantry' }] }];
  const after = attack(g, [0]);
  assert.equal(after.events.at(-1).handler.origin.Advance, 'SteelWeapons');
  assert.deepEqual(after.events.at(-1).handler.request.Payment[0].cost.default, { ore: 1 });
});

test('old naval combat modifiers do not grant Steel Weapons in later rounds', () => {
  const g = fixture('combat/ship_combat');
  for (const p of g.players.slice(0, 2)) {
    p.advances.push('SteelWeapons');
    p.action_cards = []; p.objective_cards = [];
  }
  g.dice_roll_outcomes = Array(40).fill(0);
  const after = move(g, { Movement: { Move: { units: [7, 8], destination: 'D2' } } }, 0);
  assert.equal(steelWeaponsBenefit(after, view(after), 0), null);
  let changed = false;
  for (const event of after.events ?? []) {
    const combat = event.event_type?.CombatRoundEnd?.combat;
    if (combat) {
      combat.modifiers = [...(combat.modifiers ?? []), 'SteelWeaponsAttacker', 'SteelWeaponsDefender'];
      changed = true;
    }
  }
  assert.ok(changed, 'fixture pauses between rounds');
  const continued = move(after, { Response: { Bool: false } });
  const rounds = combatJournal(journal(continued)).filter((entry) => entry.combat);
  assert.ok(rounds.length >= 2);
  for (const entry of rounds) assert.ok(![...entry.combat!.attacker.modifiers, ...entry.combat!.defender.modifiers].some((m: string) => /steel weapons/i.test(m)));
});


for (const target of ['empty city', 'settler', 'city with settler']) {
  for (const civ of ['Rome', 'Egypt', 'China']) {
    test(`${civ} battle rewards distinguish automatic victories from army combat: ${target}`, () => {
      const g = fixture('objective_cards/instant/conqueror'), p = g.players[0], enemy = g.players[1];
      p.civilization = civ;
      p.advances = ['Farming', 'Mining', 'Tactics', 'Bartering', 'Myths', 'Rituals'];
      p.resources = {culture_tokens: 4};
      p.action_cards = [5];
      p.objective_cards = [];
      enemy.objective_cards = [];
      enemy.action_cards = [];
      enemy.cities[0].city_pieces = {};
      enemy.units = target === 'empty city' ? [] : [{id: 0, position: 'C1', unit_type: 'Settler'}];
      if (target === 'settler') enemy.cities = [];
      if (civ === 'China') p.units[0].unit_type = {Leader: 'SunTzu'};
      let after = attack(g);
      if (view(after).stopMovement) after = move(after, {Movement: 'Stop'});
      const resources = after.players[0].resources;
      assert.equal(resources.gold ?? 0, (target === 'settler' ? 0 : 1) + (civ === 'Rome' ? 1 : 0));
      assert.equal(resources.mood_tokens ?? 0, civ === 'Rome' ? 1 : 0);
      assert.equal(resources.culture_tokens, civ === 'Egypt' ? 5 : 4);
      assert.ok(view(after).actionCards.find((c: any) => c.id === 5)?.action, 'Heroic General recognizes a won land battle');
      for (const unit of after.players[0].units.filter((u: any) => [0,1,2,3].includes(u.id)))
        assert.ok(unit.movement_restrictions.includes('Battle'));
    });
  }
}

for (const moreWarfare of [false, true]) {
  test(`Daring checks its extra condition after an undefended capture (more enemy Warfare=${moreWarfare})`, () => {
    const g = fixture('objective_cards/instant/conqueror');
    g.players[0].objective_cards = [16];
    g.players[0].action_cards = [];
    g.players[1].objective_cards = [];
    g.players[1].units = [];
    g.players[1].cities[0].city_pieces = {};
    if (moreWarfare) g.players[1].advances.push('Draft');
    const after = attack(g);
    assert.equal(after.events?.some((e: any) => e.event_type?.SelectObjectives?.shown_objective === 'Bold') ?? false, moreWarfare);
  });
}
