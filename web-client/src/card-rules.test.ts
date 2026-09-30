import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { journal } from './journal.ts';
import { combatJournal } from './combat-journal.ts';

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
    const prompt = after.events.at(-1).handler;
    assert.equal(prompt.origin.Advance, 'SteelWeapons');
    assert.deepEqual(prompt.request.Payment[0].cost.default, { ore: 1 });
    after = move(after, { Response: { Payment: [{ ore: 1 }] } }, 0);
    assert.equal(after.players[0].resources.ore, before - 1);
  });
}

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
