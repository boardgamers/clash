import { readableHistory } from './structured-log.ts';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const serialize = (g: any) => (typeof g === 'string' ? g : JSON.stringify(g));
const seat = (g: any) => engine.currentPlayer(serialize(g));
const view = (g: any, p = seat(g)): View =>
  JSON.parse(engine.webView(engine.stripSecret(serialize(g), p), p));
const move = (g: any, a: unknown) => JSON.parse(engine.tryMove(serialize(g), JSON.stringify(a), seat(g)));
const query = (g: any, input: unknown) =>
  JSON.parse(engine.webQuery(engine.stripSecret(serialize(g), seat(g)), seat(g), JSON.stringify(input)));
async function fixture(name: string) {
  const g = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  const npcs = JSON.parse(
    await engine.init(2, [], { civilization: 'Random' }, 'india-npcs', {}),
  ).players.slice(2);
  g.players = [...g.players.slice(0, 2), ...npcs];
  return g;
}
async function india(leader?: string) {
  let g = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'india-rules', {});
  g = move(g, { ChooseCivilization: 'India' });
  g = move(g, { ChooseCivilization: 'Rome' });
  const p = g.players[seat(g)];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 8, culture_tokens: 8 };
  p.resource_limit.food = 7;
  if (leader)
    p.units.push({ id: p.next_unit_id++, position: p.cities[0].position, unit_type: { Leader: leader } });
  return g;
}
const addAdvance = (p: any, ...advances: string[]) =>
  (p.advances = [...new Set([...p.advances, ...advances])]);

test('India offers all four advances and three leaders through faction setup', async () => {
  const g = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'india-picker', {});
  const option = view(g).civilizations!.find((c) => c.name === 'India')!;
  assert.equal(option.advances.length, 4);
  assert.deepEqual(
    option.leaders.map((l) => l.name),
    ['Maharaja Sri Gupta', 'Ashoka the Great', 'Akbar the Great'],
  );
});

test('Indian Elephants bypasses a Market only for elephants in Forest cities', async () => {
  const g = await india();
  const p = g.players[seat(g)],
    position = p.cities[0].position;
  addAdvance(p, 'Husbandry');
  const terrain = g.map.tiles.find(([pos]: string[]) => pos === position);
  terrain[1] = 'Forest';
  const offers = view(g).cityActions[0].recruits;
  assert.equal(offers.find((r) => r.type === 'Elephant')!.reason, null);
  assert.match(offers.find((r) => r.type === 'Cavalry')!.reason!, /market/i);
  const offer = query(g, { kind: 'recruit', city: position, units: { elephants: 1 }, replaced: [] });
  const recruited = move(g, offer.action);
  assert.ok(recruited.players[seat(g)].units.some((u: any) => u.unit_type === 'Elephant'));
  terrain[1] = 'Fertile';
  assert.match(view(g).cityActions[0].recruits.find((r) => r.type === 'Elephant')!.reason!, /market/i);
});

test('Peace & Poetry is an optional once-per-turn payment, not spent by previews or food payments', async () => {
  const g = await india();
  const p = g.players[seat(g)];
  addAdvance(p, 'Arts', 'Theaters');
  p.incident_tokens = 3;
  const quote = () => view(g).advances.find((a) => a.id === 'Husbandry')!;
  assert.ok(quote().payments.some((p) => p.payment.mood_tokens === 1));
  assert.deepEqual(
    quote().costGroups?.map((g) => g.amount),
    [1, 2],
  );
  assert.ok(quote().payments.some((p) => p.payment.food === 2));
  const food = move(g, quote().payments.find((p) => p.payment.food === 2)!.action);
  const next = view(food).advances.find((a) => a.id === 'Storage')!;
  const mood = next.payments.find((p) => p.payment.mood_tokens === 1)!;
  assert.ok(mood, 'Ordinary food payment preserves Peace & Poetry');
  const after = move(food, mood.action);
  assert.equal(after.players[seat(g)].event_info['Peace & Poetry'], 'used');
  assert.ok(
    !view(after)
      .advances.find((a) => a.id === 'Irrigation')!
      .payments.some((p) => p.payment.mood_tokens),
  );
  assert.throws(() =>
    move(after, { Playing: { Advance: { advance: 'Irrigation', payment: { mood_tokens: 1 } } } }),
  );
});

test('Golden Age researches a new category for culture plus resources, activates the leader city and costs no action', async () => {
  const g = await india('SriGupta');
  const index = seat(g),
    city = g.players[index].cities[0].position;
  g.actions_left = 0;
  const ability = view(g).specialActions!.find((a) => a.name === 'Golden Age')!;
  assert.ok(ability);
  let after = move(g, ability.action);
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(view(after).decision!.advanceMode, 'paid');
  assert.deepEqual(view(after).advances.find((a) => a.id === 'Writing')!.action, {
    Response: { SelectAdvance: 'Writing' },
  });
  assert.equal(view(after).advances.find((a) => a.id === 'Writing')!.costAmount, 2);
  const choices = view(after).decision!.options.map((o) => o.value);
  assert.ok(choices.includes('Writing'));
  assert.ok(!choices.includes('Husbandry'), 'Agriculture category is already started');
  after = move(after, { Response: { SelectAdvance: 'Writing' } });
  assert.match(view(after).decision!.fields[0].name, /Research cost/);
  const paid = move(after, { Response: { Payment: [{ food: 1, ideas: 1 }] } });
  assert.equal(paid.actions_left, 0);
  assert.ok(paid.players[index].advances.includes('Writing'));
  assert.equal(paid.players[index].cities.find((c: any) => c.position === city).activations, 1);
  assert.equal(paid.players[index].resources.food, 6);
  assert.equal(paid.players[index].resources.ideas, 6);
  assert.equal(
    paid.players[index].resources.culture_tokens,
    8,
    'Pay 1 culture, then gain Writing’s culture bonus',
  );
  g.players[index].resources = { culture_tokens: 1 };
  assert.ok(
    !view(g).specialActions?.some((a) => a.name === 'Golden Age'),
    'No dead-end action when research is unaffordable',
  );
  assert.ok(view(g).unavailableSpecialActions?.some((a) => a.name === 'Golden Age' && a.reason), 'owned Golden Age stays visible with a disabled reason');
});

async function influence(leader = false) {
  const g = await fixture('base/cultural_influence_instant');
  g.current_player_index = 0;
  const p = g.players[0];
  p.civilization = 'India';
  addAdvance(p, 'Myths', 'StateReligion');
  p.cities = [{ position: 'A1', mood_state: 'Happy' }];
  p.units = [{ id: 0, position: leader ? 'A1' : 'B3', unit_type: 'Settler' }];
  if (leader) p.units.push({ id: 1, position: 'A1', unit_type: { Leader: 'Ashoka' } });
  p.next_unit_id = 2;
  g.players[1].cities[0].city_pieces = { temple: 1 };
  return g;
}

test('Proselytism exposes settler origins, adds range and rejects a forged origin', async () => {
  const g = await influence();
  g.players[0].units.push({ id: 2, position: 'B3', unit_type: 'Settler' });
  const target = view(g).influence!.find((t) => t.position === 'C1' && t.name === 'Temple')!;
  const settler = target.origins!.find((o) => o.position === 'B3')!;
  assert.ok(settler.settlers);
  assert.deepEqual(settler.payment, {});
  const attempt = structuredClone(settler.action) as any;
  attempt.Playing.InfluenceCultureAttempt.starting_position = 'A2';
  assert.throws(() => move(g, attempt), /origin|range/);
  const after = move(g, settler.action);
  assert.equal(after.players[1].cities[0].city_pieces.temple, 0);
  assert.match(JSON.stringify(readableHistory(after).log), /from B3/);
  g.players[0].advances = g.players[0].advances.filter((a: string) => a !== 'StateReligion');
  assert.ok(
    !view(g)
      .influence!.find((t) => t.name === 'Temple')
      ?.origins?.some((o) => o.settlers),
  );
});

test('Buddhism offers a reroll before boosting, resolves it once, and permits declining', async () => {
  for (const reroll of [true, false]) {
    const g = await influence(true);
    g.dice_roll_outcomes = [10, 0]; // 1, then 6
    const target = view(g).influence!.find((t) => t.name === 'Temple')!;
    const origin = target.origins!.find((o) => o.position === 'A1')!;
    assert.ok(origin.reroll);
    let after = move(g, origin.action);
    assert.match(view(after).choiceDecision?.name ?? '', /Buddhism/);
    after = move(after, { Response: { Bool: reroll } });
    if (reroll) {
      assert.equal(after.players[1].cities[0].city_pieces.temple, 0);
      assert.equal(after.players[0].event_info.Buddhism, 'used');
      assert.equal(after.events?.length ?? 0, 0);
    } else {
      assert.ok(!after.players[0].event_info?.Buddhism);
      assert.equal(view(after).decision!.fields[0].initial.culture_tokens, 4);
      after = move(after, { Response: { Payment: [{ culture_tokens: 4 }] } });
      assert.equal(after.players[1].cities[0].city_pieces.temple, 0);
    }
  }
});

test('Elephant trade routes offer Prosperity mood and only one culture per influenced target', async () => {
  const g = await fixture('advances/trade_routes');
  const p = g.players[1];
  p.civilization = 'India';
  addAdvance(p, 'Husbandry', 'Currency');
  p.units = [0, 1].map((id) => ({ id, position: 'D8', unit_type: 'Elephant' }));
  p.next_unit_id = 2;
  g.map.tiles.find(([pos]: string[]) => pos === 'D7')[1] = 'Fertile';
  g.players[0].cities[0].city_pieces.market = 1;
  g.players[0].cities[1].position = 'E8';
  const after = move(g, { Playing: 'EndTurn' });
  const decision = view(after).decision!;
  assert.ok(decision.fields[0].choices!.some((c) => c.mood_tokens === 2));
  assert.ok(decision.fields[0].choices!.some((c) => c.gold === 2));
  assert.ok(decision.fields[0].choices!.some((c) => c.culture_tokens === 1 && c.mood_tokens === 1));
  assert.ok(!decision.fields[0].choices!.some((c) => (c.culture_tokens ?? 0) > 1));
  const paid = move(after, { Response: { ResourceReward: { culture_tokens: 1, mood_tokens: 1 } } });
  assert.match(JSON.stringify(paid.log), /elephant at D8 traded/);
  p.cities[0].mood_state = 'Angry';
  const angry = move(g, { Playing: 'EndTurn' });
  assert.ok(!view(angry).decision, 'Angry city elephants cannot trade');
});

test('Prepared adds combat value without a tactics card; Expansionist rewards a captured Temple city', async () => {
  for (const leader of ['SriGupta', 'Ashoka']) {
    const g = await fixture('civilizations/china/fast_war');
    g.players[0].civilization = 'India';
    g.players[0].units[0].unit_type = { Leader: leader };
    g.players[0].resources.culture_tokens = 0;
    g.players[1].cities[0].city_pieces = { temple: 1 };
    const after = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
    if (leader === 'SriGupta') assert.match(JSON.stringify(after.log), /Prepared adds \+2 combat value/);
    else assert.equal(after.players[0].resources.culture_tokens, 2);
  }
});

test('Prosperous grants gold only when Akbar moves into an owned city', async () => {
  const g = await fixture('civilizations/china/fast_war');
  const p = g.players[0];
  p.civilization = 'India';
  p.units[0].unit_type = { Leader: 'Akbar' };
  p.resources.gold = 0;
  p.cities.push({ position: 'D7', mood_state: 'Happy' });
  const after = move(g, { Movement: { Move: { units: [0], destination: 'D7', payment: {} } } });
  assert.equal(after.players[0].resources.gold, 1);
  p.cities.pop();
  const empty = move(g, { Movement: { Move: { units: [0], destination: 'D7', payment: {} } } });
  assert.equal(empty.players[0].resources.gold ?? 0, 0);
});

test('Prepared does not apply in a round where Sri Gupta plays a tactics card', async () => {
  const g = await fixture('civilizations/china/fast_war');
  g.players[0].civilization = 'India';
  g.players[0].units[0].unit_type = { Leader: 'SriGupta' };
  g.players[0].action_cards = [1];
  const choosing = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
  const after = move(choosing, { Response: { SelectHandCards: [{ ActionCard: 1 }] } });
  assert.doesNotMatch(JSON.stringify(after.log), /Prepared adds/);
});

test('Bladed Tusks persists after Akbar dies and is optional before battle', async () => {
  const g = await fixture('civilizations/china/fast_war');
  const p = g.players[0];
  p.civilization = 'India';
  p.units[0].unit_type = { Leader: 'Akbar' };
  p.resources.ore = 4;
  p.units.push(
    { id: 1, position: 'E8', unit_type: 'Elephant' },
    { id: 2, position: 'E8', unit_type: 'Elephant' },
  );
  p.next_unit_id = 3;
  g.players[1].units.push(
    { id: 1, position: 'D8', unit_type: 'Infantry' },
    { id: 2, position: 'D8', unit_type: 'Infantry' },
  );
  g.players[1].next_unit_id = 3;
  g.dice_roll_outcomes = Array(30).fill(2);
  const start = move(g, { Movement: { Move: { units: [0, 1, 2], destination: 'D8', payment: {} } } });
  assert.equal(view(start).decision!.name, 'Bladed Tusks');
  const declined = move(start, { Response: { Payment: [{}] } });
  assert.doesNotMatch(JSON.stringify(declined.log), /Bladed Tusks adds/);
  let after = move(start, { Response: { Payment: [{ ore: 1 }] } });
  assert.equal(after.players[0].resources.ore, 3);
  after = move(after, { Response: { SelectUnits: [0] } });
  assert.ok(!after.players[0].units.some((u: any) => u.unit_type?.Leader === 'Akbar'));
  after = move(after, { Response: { Bool: false } });
  const last = readableHistory(after)
    .log!.at(-1)!
    .rounds.at(-1)!
    .turns.at(-1)!
    .actions!.at(-1)!
    .log!.join(' ');
  assert.match(last, /Combat round 2/);
  assert.match(last, /Bladed Tusks adds \+2 combat value/);
});

test('Government completed by the free advance is marked ready for the next age, not claimed retroactively', async () => {
  let g = await india();
  const index = seat(g),
    p = g.players[index];
  for (const player of g.players) player.objective_cards = [];
  p.objective_cards = [24];
  addAdvance(p, 'Myths', 'StateReligion', 'Dogma', 'Devotion', 'Conversion');
  p.incident_tokens = 3;
  const government = (state: any) =>
    view(state, index).objectiveCards[0].objectives.find((o) => o.name === 'Government')!;
  assert.equal(government(g).conditionMet, false);
  for (let i = 0; i < 6; i++) g = move(g, { Playing: 'EndTurn' });
  assert.equal(view(g).objectiveDecision, null);
  assert.ok(view(g).decision?.advanceSelection);
  assert.equal(seat(g), index);
  g = move(g, { Response: { SelectAdvance: 'Fanaticism' } });
  assert.equal(government(g).conditionMet, true);
  assert.equal(government(g).scoringAge, 2);
  assert.equal(view(g, index).objectiveDecision, null);
  assert.ok(g.players[index].objective_cards.includes(24));
  assert.ok(!g.players[index].completed_objectives?.some((o: any) => o.name === 'Government'));
  const opponentView: View = JSON.parse(engine.webView(engine.stripSecret(serialize(g), 1 - index), index));
  assert.deepEqual(opponentView.objectiveCards, []);
});
