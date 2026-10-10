import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { frameDetails } from './playback.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const serialize = (g: any) => (typeof g === 'string' ? g : JSON.stringify(g));
const seat = (g: any) => engine.currentPlayer(serialize(g));
const view = (g: any, p = seat(g)): any => JSON.parse(engine.webView(engine.stripSecret(serialize(g), p), p));
const move = (g: any, a: unknown): any =>
  JSON.parse(engine.tryMove(serialize(g), JSON.stringify(a), seat(g)));
const query = (g: any, input: unknown): any =>
  JSON.parse(engine.webQuery(engine.stripSecret(serialize(g), seat(g)), seat(g), JSON.stringify(input)));
const addAdvance = (p: any, ...advances: string[]) =>
  (p.advances = [...new Set([...p.advances, ...advances])]);
async function setup(civ: string, leader?: string) {
  let g = await engine.init(
    2,
    [],
    { civilization: 'ChooseCivilization' },
    'remaining-civilizations',
    {},
  );
  g = move(g, { ChooseCivilization: civ });
  g = move(g, { ChooseCivilization: 'Rome' });
  const p = g.players[seat(g)];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 8, culture_tokens: 8 };
  p.resource_limit.food = 7;
  if (leader)
    p.units.push({ id: p.next_unit_id++, position: p.cities[0].position, unit_type: { Leader: leader } });
  return g;
}
async function fixture(name: string, civ: string, leader?: string) {
  const g = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  const npcs = JSON.parse(
    await engine.init(2, [], { civilization: 'Random' }, 'remaining-npcs', {}),
  ).players.slice(2);
  g.players = [...g.players.slice(0, 2), ...npcs];
  g.players[0].civilization = civ;
  if (leader) g.players[0].units[0].unit_type = { Leader: leader };
  return g;
}

test('Egypt and Phoenicia offer four advances and three leaders', async () => {
  const g = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'remaining-picker', {});
  for (const name of ['Egypt', 'Phoenicia']) {
    const c = view(g).civilizations.find((c: any) => c.name === name);
    assert.equal(c.advances.length, 4);
    assert.equal(c.leaders.length, 3);
  }
});
test('Flood Plains enables Barren founding and food or wood collection', async () => {
  const g = await setup('Egypt'),
    p = g.players[seat(g)];
  addAdvance(p, 'Irrigation');
  const city = p.cities[0].position;
  g.map.tiles.find(([pos]: string[]) => pos === city)[1] = 'Barren';
  const q = view(g).cities.find((c: any) => c.position === city);
  assert.match(JSON.stringify(q), /food/);
  assert.match(JSON.stringify(q), /wood/);
  const barren = g.map.tiles.find(([pos, t]: string[]) => t === 'Barren' && pos !== city)[0];
  p.units[0].position = barren;
  assert.ok(view(g).settlers.some((u: any) => u.position === barren && u.foundAction));
});
test('Man God grants abilities without adding government advances', async () => {
  const g = await setup('Egypt'),
    p = g.players[seat(g)];
  addAdvance(p, 'Myths', 'Priesthood', 'Dogma', 'Devotion', 'Conversion', 'Fanaticism');
  assert.ok(view(g).specialActions.some((a: any) => a.name === 'Absolute Power'));
  const after = move(g, { Playing: 'EndTurn' });
  assert.ok(!after.players[seat(g)].advances.includes('AbsolutePower'));
});
test('Beloved charges once and records protected city tokens', async () => {
  const g = await setup('Egypt', 'Cleopatra'),
    index = seat(g),
    city = g.players[index].cities[0].position;
  let after = move(g, view(g).specialActions.find((a: any) => a.name === 'Beloved').action);
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(after.players[index].resources.culture_tokens, 7);
  assert.equal(after.players[index].custom_data[`Beloved:${city}`].Number, 1);
  assert.ok(!view(after).specialActions.some((a: any) => a.name === 'Beloved'));
});
test('Imhotep discounts an Academy and Innovator pays once on activation', async () => {
  const g = await setup('Egypt', 'Imhotep'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Writing');
  p.resources.ideas = 0;
  p.cities.push({
    position: g.map.tiles.find(([pos, t]: string[]) => t === 'Barren' && pos !== city)[0],
    mood_state: 'Happy',
  });
  const offer = view(g)
    .cityActions.find((c: any) => c.position === city)
    .buildings.find((b: any) => b.name === 'Academy');
  assert.equal(offer.payment.food ?? 0, 0);
  assert.equal(offer.payment.wood ?? 0, 0);
  const after = move(g, offer.choices[0].action);
  assert.equal(after.players[seat(g)].resources.ideas, 3);
});
test('Architecture permits mood tokens in construction payments', async () => {
  const g = await setup('Egypt'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Engineering', 'Writing');
  p.cities.push({
    position: g.map.tiles.find(([pos, t]: string[]) => t === 'Barren' && pos !== city)[0],
    mood_state: 'Happy',
  });
  const after = move(g, {
    Playing: {
      Construct: {
        city_position: city,
        city_piece: 'Academy',
        payment: { mood_tokens: 3 },
        port_position: null,
      },
    },
  });
  assert.equal(after.players[seat(g)].resources.mood_tokens, 5);
});
test('Embalming rewards battles only at four or fewer culture', async () => {
  for (const culture of [4, 5]) {
    const g = await fixture('civilizations/china/fast_war', 'Egypt', 'Ramses');
    addAdvance(g.players[0], 'Myths', 'Rituals');
    g.players[0].resources.culture_tokens = culture;
    const after = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
    assert.equal(after.players[0].resources.culture_tokens, 5);
  }
});
test('Alphabet offers and grants two ideas for a single trade route', async () => {
  const g = await fixture('advances/trade_routes', 'Rome'),
    p = g.players[1];
  p.civilization = 'Phoenicia';
  addAdvance(p, 'Writing', 'Currency');
  p.units = [{ id: 0, position: 'D8', unit_type: 'Settler' }];
  p.next_unit_id = 1;
  g.map.tiles.find(([pos]: string[]) => pos === 'D7')[1] = 'Fertile';
  g.players[0].cities = [{ position: 'D6', mood_state: 'Happy' }];
  p.resources.ideas = 0;
  const after = move(g, { Playing: 'EndTurn' }),
    choices = view(after).choiceDecision.choices,
    ideas = choices.find((c: any) => c.pile.ideas === 2);
  assert.ok(ideas, 'one route must offer two ideas through Alphabet');
  assert.equal(choices.length, 4);
  for (const resource of ['food', 'gold', 'culture_tokens']) {
    assert.ok(choices.some((c: any) => c.pile[resource] === 1));
  }
  assert.equal(move(after, ideas.action).players[1].resources.ideas, 2);
});
test('Alphabet shares a two-route allowance between culture and double ideas', async () => {
  const g = await fixture('advances/trade_routes', 'Rome'),
    p = g.players[1];
  p.civilization = 'Phoenicia';
  addAdvance(p, 'Writing', 'Currency');
  p.units = [0, 1, 2].map((id) => ({ id, position: 'D8', unit_type: 'Settler' }));
  p.next_unit_id = 3;
  g.map.tiles.find(([pos]: string[]) => pos === 'D7')[1] = 'Fertile';
  g.players[0].cities = ['D6', 'E8', 'F8'].map((position) => ({ position, mood_state: 'Happy' }));
  const after = move(g, { Playing: 'EndTurn' }),
    choices = view(after).decision.fields[0].choices;
  assert.ok(choices.some((c: any) => c.ideas === 4 && c.food === 1));
  assert.ok(choices.some((c: any) => c.ideas === 2 && c.culture_tokens === 1 && c.gold === 1));
  assert.ok(!choices.some((c: any) => (c.ideas ?? 0) / 2 + (c.culture_tokens ?? 0) > 2));
});
test('City Independence protects Ports from influence', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Rome');
  g.current_player_index = 0;
  g.players[1].civilization = 'Phoenicia';
  addAdvance(g.players[1], 'Fishing');
  g.players[1].cities[0].city_pieces = { port: 1, temple: 1 };
  const targets = view(g).influence;
  assert.ok(targets.some((t: any) => t.name === 'Temple'));
  assert.ok(!targets.some((t: any) => t.name === 'Port'));
});

test('A Port city can trade with a coastal city three spaces away', async () => {
  const g = await fixture('advances/trade_routes', 'Rome'),
    p = g.players[1];
  p.civilization = 'Phoenicia';
  p.units = [];
  for (const t of g.map.tiles) t[1] = 'Fertile';
  for (const pos of ['D7', 'D4']) g.map.tiles.find(([p]: string[]) => p === pos)[1] = 'Water';
  p.cities = [{ position: 'D8', mood_state: 'Happy', city_pieces: { port: 1 }, port_position: 'D7' }];
  g.players[0].cities = [{ position: 'D5', mood_state: 'Happy' }];
  p.resources.food = 0;
  const after = move(g, { Playing: 'EndTurn' });
  assert.equal(after.players[1].resources.food, 1);
  assert.match(JSON.stringify(after.log), /Port city at D8 traded with city D5/);
});
test('Only Hiram can provide a fifth route', async () => {
  for (const leader of ['Hiram', 'Ithobaal']) {
    const g = await fixture('advances/trade_routes', 'Rome'),
      p = g.players[1];
    p.civilization = 'Phoenicia';
    for (const t of g.map.tiles) t[1] = 'Fertile';
    p.units = [0, 1, 2, 3, 4].map((id) => ({ id, position: 'D8', unit_type: 'Settler' }));
    p.units.push({ id: 5, position: 'D8', unit_type: { Leader: leader } });
    p.next_unit_id = 6;
    g.players[0].cities = ['C8', 'D6', 'D7', 'E8', 'F7'].map((position) => ({
      position,
      mood_state: 'Happy',
    }));
    p.resources.food = 0;
    p.resource_limit.food = 7;
    const after = move(g, { Playing: 'EndTurn' });
    assert.equal(after.players[1].resources.food, leader === 'Hiram' ? 5 : 4);
  }
});
test('Ballcourts explicitly buys an extra collection tile; Terracing offers three Mountain yields', async () => {
  const g = await setup('Maya'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Arts', 'Sports', 'Irrigation');
  p.cities[0].mood_state = 'Neutral';
  const choices = view(g).cities[0].choices,
    mountain = choices.filter(
      (c: any) => g.map.tiles.find(([pos]: string[]) => pos === c.position)[1] === 'Mountain',
    );
  assert.ok(mountain.some((c: any) => c.pile.wood === 1));
  assert.ok(mountain.some((c: any) => c.pile.food === 1));
  const selections = [
    choices.find((c: any) => c.position === city),
    choices.find((c: any) => c.position !== city),
  ].map((c: any) => ({ position: c.position, pile: c.pile, times: 1 }));
  const params = { kind: 'collect', city, selections, variant: 'Collect' };
  assert.throws(() => query(g, params));
  const after = move(g, query(g, { ...params, ballcourts: true }).action);
  assert.equal(after.players[seat(g)].resources.mood_tokens, 7);
  assert.equal(after.actions_left, g.actions_left - 1);
  p.cities[0].mood_state = 'Angry';
  assert.throws(() => query(g, { ...params, ballcourts: true }));
});
test('Ballcourts recruits extra units for one mood and the ordinary action', async () => {
  const g = await setup('Maya'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Arts', 'Sports');
  p.cities[0].mood_state = 'Neutral';
  const params = { kind: 'recruit', city, units: { infantry: 2 }, replaced: [] };
  assert.throws(() => query(g, params));
  const offer = query(g, { ...params, ballcourts: true });
  assert.equal(offer.payment.mood_tokens, 1);
  const after = move(g, offer.action);
  assert.equal(after.players[seat(g)].resources.mood_tokens, 7);
  assert.equal(after.actions_left, g.actions_left - 1);
});
test('Ballcourts discounts Spirituality once per turn, without consuming it on previews', async () => {
  const g = await setup('Maya'),
    p = g.players[seat(g)];
  addAdvance(p, 'Arts', 'Sports');
  p.incident_tokens = 3;
  const offer = view(g).advances.find((a: any) => a.id === 'Myths');
  assert.ok(offer.payments.some((p: any) => p.payment.food === 1));
  const after = move(g, offer.payments.find((p: any) => p.payment.food === 1).action);
  assert.ok(
    view(after)
      .advances.find((a: any) => a.id === 'Rituals')
      .payments.every((p: any) => Object.values(p.payment).reduce((a: any, b: any) => a + b, 0) === 2),
  );
});
test('Calendar reveals the next event privately and optionally buries it for one culture', async () => {
  const g = await setup('Maya'),
    p = g.players[seat(g)],
    index = seat(g);
  addAdvance(p, 'Fishing', 'Navigation', 'Cartography', 'Astronomy');
  const first = g.incidents_left[0];
  const start = move(g, view(g).specialActions.find((a: any) => a.name === 'Calendar').action);
  assert.match(view(start).decision.fields[0].name, /bottom/);
  const hidden = JSON.parse(engine.stripSecret(serialize(start), 1 - index));
  const serialized = JSON.stringify(hidden.events);
  assert.match(serialized, /Calendar · Inspecting the next event/);
  assert.doesNotMatch(serialized, /Pay 1 culture to put this event/);
  const kept = move(start, { Response: { Payment: [{}] } });
  assert.equal(kept.incidents_left[0], first);
  assert.equal(kept.players[index].resources.culture_tokens, 8);
  const buried = move(start, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(buried.incidents_left.at(-1), first);
  assert.equal(buried.players[index].resources.culture_tokens, 7);
  p.resources.culture_tokens = 0;
  const noTokens = move(g, view(g).specialActions.find((a: any) => a.name === 'Calendar').action);
  assert.deepEqual(view(noTokens).decision.fields[0].choices, [{}]);
});
test('Cultural Influx offers free influence after construction', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Maya');
  g.current_player_index = 0;
  const p = g.players[0];
  p.cities = [
    { position: 'A1', mood_state: 'Happy' },
    { position: 'B3', mood_state: 'Happy' },
  ];
  p.units = [{ id: 0, position: 'A1', unit_type: { Leader: 'SiyajKak' } }];
  p.next_unit_id = 1;
  p.resources = { food: 7, wood: 7, ore: 7, culture_tokens: 7 };
  addAdvance(p, 'Writing');
  g.players[1].cities[0].city_pieces = { temple: 1 };
  g.dice_roll_outcomes = [10];
  const offer = view(g)
    .cityActions.find((c: any) => c.position === 'A1')
    .buildings.find((b: any) => b.name === 'Academy');
  const built = move(g, offer.choices[0].action);
  assert.match(view(built).decision.description, /Cultural Influx/);
  const after = move(built, {
    Response: { SelectStructures: [{ position: 'C1', structure: { Building: 'Temple' } }] },
  });
  assert.equal(after.players[1].cities[0].city_pieces.temple, 0);
  assert.equal(after.actions_left, g.actions_left - 1);
});
test('Reconstruction replaces a building after movement and keeps the city Angry', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Maya', 'WakChanilAjaw');
  g.players[0].advances = ['Farming', 'Mining', 'Writing', 'Tactics'];
  g.players[1].cities[0].city_pieces = { temple: 1 };
  let after = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
  if (!view(after).decision) after = move(after, { Movement: 'Stop' });
  assert.match(view(after).decision.description, /Reconstruction/);
  after = move(after, {
    Response: { SelectStructures: [{ position: 'D8', structure: { Building: 'Temple' } }] },
  });
  after = move(after, {
    Response: { SelectStructures: [{ position: 'D8', structure: { Building: 'Academy' } }] },
  });
  const city = after.players[0].cities.find((c: any) => c.position === 'D8');
  assert.equal(city.mood_state, 'Angry');
  assert.equal(city.city_pieces.academy, 0);
  assert.ok(city.city_pieces.temple == null);
});

test('Persian Elephants recruit beside Barren land without a Market', async () => {
  const g = await setup('Persia'),
    p = g.players[seat(g)];
  addAdvance(p, 'Husbandry');
  assert.equal(view(g).cityActions[0].recruits.find((r: any) => r.type === 'Elephant').reason, null);
  for (const t of g.map.tiles) if (t[1] === 'Barren') t[1] = 'Forest';
  assert.match(view(g).cityActions[0].recruits.find((r: any) => r.type === 'Elephant').reason, /market/i);
});
test('Zoroastrianism converts an army unit through the influence controls and shares the success limit', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Persia');
  g.current_player_index = 0;
  const p = g.players[0];
  addAdvance(p, 'Myths', 'Priesthood');
  p.resources.culture_tokens = 7;
  p.units = [];
  g.players[1].units = [{ id: 0, position: 'B3', unit_type: 'Infantry' }];
  g.players[1].next_unit_id = 1;
  g.dice_roll_outcomes = [10];
  const offer = view(g).influence.find((i: any) => i.variant === 'Zoroastrianism');
  assert.ok(offer);
  const after = move(g, offer.action);
  assert.ok(after.players[0].units.some((u: any) => u.position === 'B3' && u.unit_type === 'Infantry'));
  assert.equal((after.players[1].units ?? []).length, 0);
  assert.equal(view(after).influence.length, 0);
  assert.equal(after.actions_left, g.actions_left - 1);
  g.players[1].civilization = 'Rome';
  g.players[1].units[0].unit_type = { Leader: 'Caesar' };
  assert.ok(!view(g).influence.some((i: any) => i.variant === 'Zoroastrianism'));
});
test('Darius gains mood when influence from his city succeeds', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Persia');
  g.current_player_index = 0;
  const p = g.players[0];
  p.units = [{ id: 0, position: 'A1', unit_type: { Leader: 'Darius' } }];
  p.resources.mood_tokens = 0;
  g.players[1].cities[0].city_pieces = { temple: 1 };
  g.dice_roll_outcomes = [10];
  const offer = view(g).influence.find((i: any) => i.name === 'Temple');
  assert.ok(offer);
  const from = offer.origins.find((o: any) => o.position === 'A1');
  let after = move(g, from.action);
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(after.players[0].resources.mood_tokens, 1);
});
test('Architect purchases a building for two culture and normal resources, without an action', async () => {
  const g = await setup('Persia', 'Darius'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Writing');
  const extra = g.map.tiles.find(([pos, t]: string[]) => t === 'Forest' && pos !== city)[0];
  p.cities.push({ position: extra, mood_state: 'Happy' });
  g.actions_left = 0;
  const offer = view(g).specialActions.find((a: any) => a.name === 'Architect');
  assert.ok(offer);
  let after = move(g, offer.action);
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 2 }] } });
  if (view(after).decision?.options.length)
    after = move(after, {
      Response: { SelectStructures: [{ position: city, structure: { Building: 'Academy' } }] },
    });
  after = move(after, { Response: { Payment: [{ food: 1, wood: 1, ore: 1 }] } });
  assert.equal(after.actions_left, 0);
  assert.equal(after.players[seat(g)].resources.culture_tokens, 6);
  assert.equal(after.players[seat(g)].cities[0].city_pieces.academy, seat(g));
  assert.ok(!view(after).specialActions.some((a: any) => a.name === 'Architect'));
});
test('Banking resolves before Trade Routes and requires existing gold', async () => {
  const g = await fixture('advances/trade_routes', 'Rome'),
    p = g.players[1];
  p.civilization = 'Persia';
  addAdvance(p, 'Currency');
  p.resources.gold = 1;
  g.map.tiles.find(([pos]: string[]) => pos === 'D7')[1] = 'Fertile';
  let after = move(g, { Playing: 'EndTurn' });
  assert.match((view(after).decision ?? view(after).choiceDecision).name, /Banking/);
  after = move(after, { Response: { ResourceReward: { culture_tokens: 1 } } });
  assert.match((view(after).decision ?? view(after).choiceDecision).name, /trade routes/i);
  p.resources.gold = 0;
  assert.match(
    (view(move(g, { Playing: 'EndTurn' })).decision ?? view(move(g, { Playing: 'EndTurn' })).choiceDecision)
      .name,
    /trade routes/i,
  );
});
test('Immortals offers a bounded payment each combat round', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Persia', 'Cyrus'),
    p = g.players[0];
  addAdvance(p, 'Draft');
  p.resources.culture_tokens = 7;
  const before = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
  const field = view(before).decision.fields[0];
  assert.equal(view(before).decision.name, 'Immortals');
  assert.deepEqual(field.choices.map((c: any) => c.culture_tokens ?? 0).sort(), [0, 1, 2, 3, 4]);
  const after = move(before, { Response: { Payment: [{ culture_tokens: 3 }] } });
  assert.match(JSON.stringify(after.log), /Immortals adds \+3/);
  assert.equal(after.players[0].resources.culture_tokens, 4);
});
test('Xerxes moves with four other army units, while normal stacks remain limited to four', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Persia', 'Xerxes'),
    p = g.players[0];
  p.units = [
    { id: 0, position: 'E8', unit_type: { Leader: 'Xerxes' } },
    ...[1, 2, 3, 4].map((id) => ({ id, position: 'E8', unit_type: 'Infantry' })),
  ];
  p.next_unit_id = 5;
  const after = move(g, { Movement: { Move: { units: [0, 1, 2, 3, 4], destination: 'D7', payment: {} } } });
  assert.equal(after.players[0].units.filter((u: any) => u.position === 'D7').length, 5);
  p.units[0].unit_type = { Leader: 'Cyrus' };
  assert.throws(() =>
    move(g, { Movement: { Move: { units: [0, 1, 2, 3, 4], destination: 'D7', payment: {} } } }),
  );
});

test('Japan offers Pottery, cavalry recruitment at Fortresses, and three leaders', async () => {
  const g = await setup('Japan'),
    p = g.players[seat(g)];
  addAdvance(p, 'Storage', 'Husbandry', 'Tactics');
  p.cities[0].city_pieces = { fortress: seat(g) };
  assert.equal(view(g).cityActions[0].recruits.find((r: any) => r.type === 'Cavalry').reason, null);
  p.cities[0].city_pieces = {};
  assert.match(view(g).cityActions[0].recruits.find((r: any) => r.type === 'Cavalry').reason, /market/i);
});
test('Shogunate Draft may recruit a card alone and does not trigger Nationalism', async () => {
  const g = await setup('Japan'),
    p = g.players[seat(g)],
    index = seat(g),
    city = p.cities[0].position;
  addAdvance(p, 'Tactics', 'Draft', 'Nationalism');
  p.cities[0].mood_state = 'Neutral';
  const before = p.action_cards.length,
    offer = query(g, { kind: 'recruit', city, units: {}, replaced: [], draftCard: true });
  assert.equal(offer.payment.mood_tokens, 1);
  const after = move(g, offer.action);
  assert.equal(after.players[index].action_cards.length, before + 1);
  assert.equal(after.players[index].resources.mood_tokens, 7);
  assert.equal(after.actions_left, g.actions_left - 1);
  assert.equal(view(after).decision, null);
  assert.ok(!view(after).cities[0].shogunateDraft);
  assert.throws(() => query(after, { kind: 'recruit', city, units: {}, replaced: [], draftCard: true }));
});
test('Shogunate Draft combines a card and regular units without a second Draft discount', async () => {
  const g = await setup('Japan'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Tactics', 'Draft', 'Nationalism');
  const offer = query(g, { kind: 'recruit', city, units: { infantry: 1 }, replaced: [], draftCard: true });
  assert.deepEqual(offer.payment, { food: 1, ore: 1, mood_tokens: 1 });
  const after = move(g, offer.action);
  assert.equal(after.players[seat(g)].action_cards.length, p.action_cards.length + 1);
});
test('Japanese Buddhism reclaims a building and State Religion removes only the action cost', async () => {
  for (const free of [false, true]) {
    const g = await setup('Japan', 'Suiko'),
      p = g.players[seat(g)],
      index = seat(g),
      city = p.cities[0].position;
    p.cities[0].city_pieces = { academy: 1 - index };
    if (free) addAdvance(p, 'Dogma', 'StateReligion');
    const offer = view(g).specialActions.find((a: any) => a.name === 'Japanese Buddhism');
    assert.ok(offer);
    let after = move(g, offer.action);
    if (view(after).decision?.fields.length)
      after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
    if (view(after).decision)
      after = move(after, {
        Response: { SelectStructures: [{ position: city, structure: { Building: 'Academy' } }] },
      });
    assert.equal(after.players[index].cities[0].city_pieces.academy, index);
    assert.equal(after.players[index].resources.culture_tokens, 7);
    assert.equal(after.actions_left, g.actions_left - Number(!free));
  }
});
test('Subterfuge discards both cards while leaving the cancelled card costs unpaid', async () => {
  const g = await setup('Greece'),
    index = seat(g),
    p = g.players[index],
    opponent = g.players[1 - index];
  opponent.civilization = 'Japan';
  addAdvance(opponent, 'Tactics');
  opponent.units[0].position = p.cities[0].position;
  opponent.units[0].unit_type = 'Infantry';
  p.action_cards = [27];
  opponent.action_cards = [9];
  const declared = move(g, { Playing: { ActionCard: 27 } });
  assert.equal(seat(declared), 1 - index);
  assert.match(view(declared).decision.description, /Subterfuge/);
  const after = move(declared, { Response: { SelectHandCards: [{ ActionCard: 9 }] } });
  assert.equal(after.players[index].resources.culture_tokens, 8);
  assert.equal(after.actions_left, g.actions_left);
  assert.equal((after.players[index].action_cards ?? []).length, 0);
  assert.equal((after.players[1 - index].action_cards ?? []).length, 0);
  assert.ok(!JSON.stringify(after.permanent_effects ?? []).includes('Assassination'));
});
test('Shogunate offers an optional free action and uses the allowance only after the card is accepted', async () => {
  const g = await setup('Japan'),
    p = g.players[seat(g)];
  addAdvance(p, 'Nationalism', 'Writing');
  p.action_cards = [17];
  p.cities.push({ position: 'C3', mood_state: 'Happy' });
  const declared = move(g, { Playing: { ActionCard: 17 } });
  assert.match(view(declared).choiceDecision.name, /Shogunate/);
  let after = move(declared, { Response: { Bool: true } });
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(after.players[seat(g)].event_info['Shogunate card'], 'used');
  assert.equal(after.actions_left, g.actions_left + 1); // City Development reserves the later building action.
});

test('Nomads moves a city with units, pays one action, activates it, and gives the terrain resource', async () => {
  const g = await setup('Huns'),
    p = g.players[seat(g)],
    index = seat(g),
    city = p.cities[0].position;
  addAdvance(p, 'Storage');
  p.resources.wood = 0;
  const offer = query(g, { kind: 'movement', city, units: [p.units[0].id] }).destinations.find(
    (d: any) => d.terrain === 'Forest',
  );
  assert.ok(offer);
  const after = move(g, offer.action);
  assert.equal(after.players[index].cities[0].position, offer.position);
  assert.equal(after.players[index].units[0].position, offer.position);
  assert.equal(after.players[index].cities[0].activations, 1);
  assert.equal(after.players[index].resources.wood, 1);
  assert.equal(after.actions_left, g.actions_left - 1);
  if (view(after).stopMovement)
    assert.equal(query(after, { kind: 'movement', city: offer.position, units: [] }).destinations.length, 0);
  else assert.equal(after.state, 'Playing');
});
test('Nomads cities can move alone, explore, and obey Mountain restrictions across Move actions', async () => {
  const g = await setup('Huns'),
    p = g.players[seat(g)],
    index = seat(g),
    city = p.cities[0].position;
  addAdvance(p, 'Storage');
  p.units = [];
  const offer = query(g, { kind: 'movement', city, units: [] }).destinations.find(
    (d: any) => d.terrain === 'Mountain',
  );
  assert.ok(offer);
  let after = move(g, offer.action);
  assert.equal(after.players[index].cities[0].nomad_mountain, true);
  if (view(after).stopMovement) after = move(after, { Movement: 'Stop' });
  if (view(after).stopMovement)
    assert.equal(query(after, { kind: 'movement', city: offer.position, units: [] }).destinations.length, 0);
  else assert.equal(after.state, 'Playing');
  const explore = query(g, { kind: 'movement', city, units: [] }).destinations.find(
    (d: any) => d.terrain === 'Unexplored',
  );
  // Reveal a neighboring map block using a city as the only mover.
  if (explore) {
    let revealed = move(g, explore.action);
    if (view(revealed).explorationDecision) {
      const choice = view(revealed).explorationDecision;
      assert.ok(choice);
    } else assert.equal(revealed.players[index].cities[0].position, explore.position);
  }
});
test('Mounted Archers grants movement only to new Cavalry and cannot move cities', async () => {
  const g = await setup('Huns'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  addAdvance(p, 'Husbandry', 'Storage', 'Tactics');
  const offer = query(g, { kind: 'recruit', city, units: { cavalry: 1 }, replaced: [] });
  const after = move(g, offer.action);
  assert.ok(after.state.Movement.unit_only);
  assert.equal(view(after).nomadCities.length, 0);
  assert.equal(query(after, { kind: 'movement', units: [p.units[0].id] }).destinations.length, 0);
});
test('Attila increases size-one capacity; Bleda can choose one Happy activation without changing mood', async () => {
  const g = await setup('Huns', 'Attila');
  assert.equal(view(g).cities[0].capacity, 3);
  const b = await setup('Huns', 'Bleda'),
    p = b.players[seat(b)];
  p.cities[0].mood_state = 'Angry';
  const offer = view(b).specialActions.find((a: any) => a.name === 'Enforcer');
  assert.ok(offer);
  const ready = move(b, offer.action);
  assert.equal(view(ready).cities[0].capacity, 2);
  const city = ready.players[seat(b)].cities[0].position;
  const recruited = move(
    ready,
    query(ready, { kind: 'recruit', city, units: { infantry: 2 }, replaced: [] }).action,
  );
  assert.equal(recruited.players[seat(b)].cities[0].mood_state, 'Angry');
  assert.equal(recruited.players[seat(b)].cities[0].angry_activation, true);
});
test('Raiders blocks unprotected enemy settlers from founding, but an escort restores founding', async () => {
  const g = await setup('Greece'),
    p = g.players[seat(g)],
    enemy = g.players[1 - seat(g)];
  enemy.civilization = 'Huns';
  addAdvance(enemy, 'Tactics');
  const target = g.map.tiles.find(([pos, t]: string[]) => t === 'Forest' && pos !== p.cities[0].position)[0];
  p.units[0].position = target;
  enemy.units = [{ id: 0, position: p.cities[0].position, unit_type: 'Infantry' }];
  assert.ok(!view(g).settlers[0].foundAction);
  p.units.push({ id: p.next_unit_id++, position: target, unit_type: 'Infantry' });
  assert.ok(view(g).settlers[0].foundAction);
});

test('Druidic Influence marks a Barbarian city, awards its point, and leaves ownership unchanged', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Celts');
  g.current_player_index = 0;
  const p = g.players[0];
  addAdvance(p, 'Myths', 'Priesthood');
  g.players[2].cities = [{ position: 'B1', mood_state: 'Neutral' }];
  g.dice_roll_outcomes = [10];
  const offer = view(g).influence.find((i: any) => i.position === 'B1');
  assert.ok(offer);
  const after = move(g, offer.action);
  assert.equal(after.players[2].cities[0].influence_marker, 0);
  assert.ok(!after.players[0].cities.some((c: any) => c.position === 'B1'));
  assert.ok(
    after.players[0].special_victory_points.some(
      (v: any) => v.points === 1 && v.attribution === 'Objectives',
    ),
  );
  after.successful_cultural_influence = false;
  assert.ok(!view(after).influence.some((i: any) => i.position === 'B1'));
});
test('Tribal Trade allows Barbarian routes and Currency cannot turn those rewards into gold', async () => {
  const g = await fixture('advances/trade_routes', 'Rome'),
    p = g.players[1];
  p.civilization = 'Celts';
  addAdvance(p, 'Currency');
  for (const t of g.map.tiles) t[1] = 'Fertile';
  g.players[0].cities = [];
  g.players[2].cities = [{ position: 'D7', mood_state: 'Neutral' }];
  p.units = [{ id: 0, position: 'D8', unit_type: 'Settler' }];
  p.resources.food = 0;
  const after = move(g, { Playing: 'EndTurn' });
  assert.equal(after.players[1].resources.food, 1);
  assert.match(JSON.stringify(after.log), /traded with city D7/);
});
test('Great Numbers retains a Settler after founding for one food', async () => {
  const g = await setup('Celts', 'Boudica'),
    p = g.players[seat(g)],
    index = seat(g);
  const pos = g.map.tiles.find(([pos, t]: string[]) => t === 'Forest' && pos !== p.cities[0].position)[0];
  for (const u of p.units) u.position = pos;
  const start = move(g, { Playing: { FoundCity: { settler: 0 } } });
  assert.match(view(start).decision.fields[0].name, /Great Numbers/);
  const after = move(start, { Response: { Payment: [{ food: 1 }] } });
  assert.ok(after.players[index].units.some((u: any) => u.position === pos && u.unit_type === 'Settler'));
  assert.equal(after.players[index].resources.food, 6);
});
test('Tribal Allies offers the normal Barbarian spawn placement as a once-per-turn action', async () => {
  const g = await setup('Celts'),
    p = g.players[seat(g)];
  addAdvance(p, 'Tactics', 'Draft');
  for (const t of g.map.tiles) t[1] = 'Fertile';
  const offer = view(g).specialActions.find((a: any) => a.name === 'Tribal Allies');
  assert.ok(offer);
  const after = move(g, offer.action);
  assert.match(view(after).decision.description, /Barbarian city/);
  assert.equal(after.actions_left, g.actions_left - 1);
});
test('Rapid Recruitment offers direct deployment of each new land unit', async () => {
  const g = await setup('Celts', 'Vercingetorix'),
    p = g.players[seat(g)],
    city = p.cities[0].position;
  const after = move(g, query(g, { kind: 'recruit', city, units: { infantry: 1 }, replaced: [] }).action);
  assert.match(view(after).decision.description, /Rapid Recruitment/);
  const target = view(after).decision.options[0].position;
  assert.ok(target);
  const deployed = move(after, { Response: { SelectPositions: [target] } });
  assert.ok(
    deployed.players[seat(g)].units.some((u: any) => u.unit_type === 'Infantry' && u.position === target),
  );
});

function heldCaptives(g: any, index: number, n: number) {
  const p = g.players[index],
    owner = 1 - index;
  p.captives = Array.from({ length: n }, (_, id) => ({ owner, unit_type: 'Infantry', id }));
  p.resources.captives = n;
  g.players[owner].held_units = { infantry: n };
}
test('all fifteen civilizations are selectable with four advances and three leaders', async () => {
  const g = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'all-fifteen', {});
  const civs = view(g).civilizations;
  assert.equal(civs.length, 15);
  assert.ok(civs.every((c: any) => c.advances.length === 4 && c.leaders.length === 3));
});
test('Aztecs hold defeated army pieces, score them, and reserve the opponents supply', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Aztecs', 'Marqzen');
  let after = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
  const decision = view(after).decision ?? view(after).choiceDecision;
  assert.match(decision.description, /Captives/);
  after = move(after, query(after, { kind: 'decision', values: [decision.options[0].value] }).action);
  assert.equal(after.players[0].captives.length, 1);
  assert.equal(after.players[0].resources.captives, 1);
  assert.equal(after.players[1].held_units.infantry, 1);
  assert.equal(view(after).players[0].scoreParts.find((v: any) => v.name === 'Objectives').points, 0.5);
});
test('Human Sacrifice allows captives to pay both leader tokens and returns pieces to their owner', async () => {
  const g = await setup('Aztecs'),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Tactics', 'Myths', 'Rituals');
  heldCaptives(g, index, 2);
  p.resources.mood_tokens = 0;
  p.resources.culture_tokens = 0;
  const after = move(g, {
    Playing: {
      Recruit: {
        city_position: p.cities[0].position,
        units: { leader: 'Marqzen' },
        payment: { captives: 2 },
      },
    },
  });
  assert.equal(after.players[index].resources.captives ?? 0, 0);
  assert.equal(after.players[index].captives?.length ?? 0, 0);
  assert.equal(after.players[1 - index].held_units?.infantry ?? 0, 0);
  assert.ok(after.players[index].units.some((u: any) => u.unit_type.Leader === 'Marqzen'));
});
test('Human Sacrifice offers event replacement before any icon or text resolves', async () => {
  const g = await setup('Aztecs'),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Tactics', 'Myths', 'Rituals');
  heldCaptives(g, index, 2);
  p.incident_tokens = 1;
  let after = move(g, view(g).advances.find((a: any) => a.id === 'Storage').action);
  assert.match(view(after).choiceDecision.name, /Human Sacrifice/);
  const before = after.incidents_discarded.length;
  after = move(after, { Response: { Bool: true } });
  assert.equal(after.players[index].resources.captives, 1);
  assert.equal(after.incidents_discarded.length, before + 1);
  assert.match(view(after).choiceDecision.name, /Human Sacrifice/);
  assert.match(JSON.stringify(after.log), /entire event and its icon are cancelled/);
});
test('Tribute Empire offers enemy cities at range two without Husbandry and takes Barbarian captives', async () => {
  const g = await fixture('base/cultural_influence_instant', 'Aztecs'),
    p = g.players[0];
  g.current_player_index = 0;
  addAdvance(p, 'Tactics', 'Voting');
  p.cities = [{ position: 'C2', mood_state: 'Happy' }];
  for (const t of g.map.tiles) t[1] = 'Fertile';
  const barb = g.players.find((p: any) => p.civilization === 'Barbarians');
  barb.cities = [{ position: 'C4', mood_state: 'Angry' }];
  barb.units = [{ id: 0, position: 'C4', unit_type: 'Infantry' }];
  barb.next_unit_id = 1;
  const after = move(g, {
    Playing: {
      Collect: {
        city_position: 'C2',
        collections: [{ position: 'C4', pile: { captives: 1 }, times: 1 }],
        action_type: 'Collect',
      },
    },
  });
  assert.equal(after.players[0].captives.length, 1);
  assert.equal(after.players[barb.id].units?.length ?? 0, 0);
  assert.equal(after.players[barb.id].held_units.infantry, 1);
});
test('Growth can spend a captive as two construction resources', async () => {
  const g = await setup('Aztecs', 'Acamapichtli'),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Tactics', 'Writing');
  heldCaptives(g, index, 1);
  const city = p.cities[0].position;
  p.cities.push({
    position: g.map.tiles.find(([pos, t]: string[]) => pos !== city && t === 'Forest')[0],
    mood_state: 'Happy',
  });
  const after = move(g, {
    Playing: { Construct: { city_position: city, city_piece: 'Academy', payment: { captives: 1, ore: 1 } } },
  });
  assert.equal(after.players[index].cities[0].city_pieces.academy, index);
  assert.equal(after.players[index].captives?.length ?? 0, 0);
});
async function carthageSea(leader?: string) {
  const g = await setup('Carthage', leader),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Fishing', 'Cartography', 'Navigation', 'Tactics');
  for (const t of g.map.tiles) t[1] = 'Water';
  p.cities = [{ position: 'D2', mood_state: 'Happy', city_pieces: { port: index }, port_position: 'D3' }];
  g.map.tiles.find(([pos]: string[]) => pos === 'D2')[1] = 'Fertile';
  p.units = [{ id: 0, position: 'D3', unit_type: 'Ship' }];
  p.next_unit_id = 1;
  if (leader) {
    p.units[0].carried_units = [{ id: 1, unit_type: { Leader: leader } }];
    p.next_unit_id = 2;
  }
  const pirates = g.players.find((p: any) => p.civilization === 'Pirates');
  pirates.units = [{ id: 0, position: 'D4', unit_type: 'Ship' }];
  pirates.next_unit_id = 1;
  return g;
}
async function piratePassengers(ownPassengers = false) {
  const g = await carthageSea(),
    index = seat(g),
    p = g.players[index];
  p.action_cards = [];
  p.units = [
    {
      id: 0,
      position: 'D3',
      unit_type: 'Ship',
      carried_units: ownPassengers
        ? [
            { id: 1, unit_type: 'Settler' },
            { id: 2, unit_type: 'Infantry' },
          ]
        : [],
    },
    {
      id: 3,
      position: 'D3',
      unit_type: 'Ship',
      pirate: true,
      carried_units: [
        { id: 4, unit_type: 'Infantry' },
        { id: 5, unit_type: 'Cavalry' },
      ],
    },
  ];
  p.next_unit_id = 6;
  const pirates = g.players.find((p: any) => p.civilization === 'Pirates');
  pirates.units = [];
  pirates.held_units = { ships: 1 };
  g.dice_roll_outcomes = [2, 11];
  return g;
}
function settlePirateRewards(g: any) {
  for (let i = 0; i < 4; i++) {
    const reward = g.events?.at(-1)?.handler?.request?.ResourceReward;
    if (!reward) return g;
    g = move(g, { Response: { ResourceReward: reward.reward.default } });
  }
  throw new Error('Pirate rewards did not finish');
}
test('Pirate passengers survive an attack on their carrier from the same sea space', async () => {
  const g = await piratePassengers(),
    index = seat(g);
  const attack = query(g, { kind: 'movement', units: [0] }).destinations.find(
    (d: any) => d.attack && d.position === 'D3',
  );
  assert.ok(attack, 'the map offers attacking an occupied allied pirate carrier');
  const after = settlePirateRewards(move(g, attack.action));
  assert.equal(after.actions_left, g.actions_left - 1);
  assert.deepEqual(
    after.players[index].units
      .find((u: any) => u.id === 0)
      .carried_units.map((u: any) => u.id)
      .sort(),
    [4, 5],
  );
  assert.ok(!after.players[index].units.some((u: any) => u.pirate || u.carrier_id != null));
  assert.equal(after.players.find((p: any) => p.civilization === 'Pirates').units?.length ?? 0, 0);
});
test('Pirate passenger overflow lets the player choose losses and survives save/reload', async () => {
  const g = await piratePassengers(true),
    index = seat(g);
  let after = move(
    g,
    query(g, { kind: 'movement', units: [0] }).destinations.find((d: any) => d.attack && d.position === 'D3')
      .action,
  );
  after = settlePirateRewards(after);
  const pending = view(after).decision ?? view(after).choiceDecision;
  assert.match(pending.description, /carried units/i);
  assert.equal(
    pending.options.length,
    4,
    'all friendly passengers can be chosen, including those from the pirates',
  );
  after = structuredClone(after);
  assert.equal(view(after).units.filter((u: any) => [1, 2, 4, 5].includes(u.id)).length, 4);
  const action = query(after, { kind: 'decision', values: [1, 2] }).action;
  assert.ok(action);
  after = move(after, action);
  assert.deepEqual(
    after.players[index].units
      .find((u: any) => u.id === 0)
      .carried_units.map((u: any) => u.id)
      .sort(),
    [4, 5],
  );
  assert.ok(!after.players[index].units.some((u: any) => u.carrier_id != null));
});
test('Pirate passengers survive a winning fleet arriving from another sea space', async () => {
  const g = await piratePassengers(),
    index = seat(g);
  g.players[index].units[0].position = 'D4';
  const attack = query(g, { kind: 'movement', units: [0] }).destinations.find(
    (d: any) => d.attack && d.position === 'D3',
  );
  assert.ok(attack);
  const after = settlePirateRewards(move(g, attack.action));
  const ship = after.players[index].units.find((u: any) => u.id === 0);
  assert.equal(ship.position, 'D3');
  assert.equal(ship.carried_units.length, 2);
});
test('Pirate passengers remain owned and visible across an interrupted naval battle', async () => {
  const g = await piratePassengers(),
    index = seat(g);
  g.dice_roll_outcomes = [2, 11, 2, 2];
  const attack = query(g, { kind: 'movement', units: [0] }).destinations.find(
    (d: any) => d.attack && d.position === 'D3',
  );
  const pending = move(g, attack.action);
  assert.equal(pending.events[0].event_type.CombatRoundEnd.combat.retreat, 'CannotRetreat');
  assert.equal(view(pending).units.filter((u: any) => [4, 5].includes(u.id)).length, 2);
  const after = settlePirateRewards(structuredClone(pending));
  assert.deepEqual(after.players[index].units[0].carried_units.map((u: any) => u.id).sort(), [4, 5]);
});
test('Pirate passengers are lost if the attacking fleet has no surviving transport', async () => {
  const g = await piratePassengers(),
    index = seat(g);
  g.dice_roll_outcomes = [11, 2];
  const after = settlePirateRewards(
    move(
      g,
      query(g, { kind: 'movement', units: [0] }).destinations.find(
        (d: any) => d.attack && d.position === 'D3',
      ).action,
    ),
  );
  assert.equal(after.players[index].units?.length ?? 0, 0);
  assert.equal(after.players.find((p: any) => p.civilization === 'Pirates').units.length, 1);
  assert.match(JSON.stringify(after.log), /carried units/i);
});
test('Pirate passengers are rescued after recruiting a ship to attack at a Port', async () => {
  const g = await piratePassengers(),
    index = seat(g),
    p = g.players[index];
  p.units = p.units.filter((u: any) => u.pirate);
  const offer = query(g, {
    kind: 'recruit',
    city: 'D2',
    units: { ships: 1 },
    replaced: [],
    attackPirates: true,
  });
  assert.ok(offer.action);
  const after = settlePirateRewards(move(g, offer.action));
  assert.equal(after.players[index].units.length, 1);
  assert.equal(after.players[index].units[0].carried_units.length, 2);
});
test('Pirate passengers are removed after a Port attack loses every ship', async () => {
  const g = await piratePassengers(),
    index = seat(g),
    p = g.players[index];
  p.units = p.units.filter((u: any) => u.pirate);
  g.dice_roll_outcomes = [11, 2];
  const offer = query(g, {
    kind: 'recruit',
    city: 'D2',
    units: { ships: 1 },
    replaced: [],
    attackPirates: true,
  });
  const after = settlePirateRewards(move(g, offer.action));
  assert.equal(after.players[index].units?.length ?? 0, 0);
});
test('Warbeasts replaces a resource per Elephant and Mercenaries drafts ships', async () => {
  const g = await setup('Carthage'),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Husbandry');
  const city = p.cities[0].position;
  const before = structuredClone(g);
  assert.throws(() =>
    move(g, {
      Playing: { Recruit: { city_position: city, units: { elephants: 1 }, payment: { food: 2, ore: 1 } } },
    }),
  );
  const elephant = move(before, {
    Playing: {
      Recruit: { city_position: city, units: { elephants: 1 }, payment: { food: 1, culture_tokens: 1 } },
    },
  });
  assert.ok(elephant.players[index].units.some((u: any) => u.unit_type === 'Elephant'));
  const sea = await carthageSea(),
    sp = sea.players[seat(sea)];
  addAdvance(sp, 'Draft');
  const ship = move(sea, {
    Playing: { Recruit: { city_position: 'D2', units: { ships: 1 }, payment: { gold: 1 } } },
  });
  assert.equal(ship.players[seat(sea)].units.filter((u: any) => u.unit_type === 'Ship').length, 2);
});
test('Pirate Allies offers peaceful entry and attack, holds extra ships, and releases them when unescorted', async () => {
  const g = await carthageSea(),
    index = seat(g);
  const q = query(g, { kind: 'movement', units: [0] });
  const targets = q.destinations.filter((d: any) => d.position === 'D4');
  assert.ok(targets.some((d: any) => d.attack));
  assert.ok(targets.some((d: any) => !d.attack));
  let after = move(g, targets.find((d: any) => !d.attack).action);
  let p = after.players[index];
  assert.equal(p.units.filter((u: any) => u.pirate).length, 1);
  assert.equal(after.players.find((p: any) => p.civilization === 'Pirates').held_units.ships, 1);
  const pirate = p.units.find((u: any) => u.pirate);
  assert.equal(query(after, { kind: 'movement', units: [pirate.id] }).destinations.length, 0);
  const next = query(after, { kind: 'movement', units: [0] }).destinations.find(
    (d: any) => d.position === 'D5' && !d.attack,
  );
  after = move(after, next.action);
  assert.equal(after.players[index].units.filter((u: any) => u.pirate).length, 0);
  assert.equal(after.players.find((p: any) => p.civilization === 'Pirates').units[0].position, 'D4');
});
test('Carthage land units may board a neutral pirate and move it with them', async () => {
  const g = await carthageSea(),
    index = seat(g),
    p = g.players[index];
  p.units = [{ id: 0, position: 'D2', unit_type: 'Settler' }];
  g.players.find((p: any) => p.civilization === 'Pirates').units[0].position = 'D3';
  const boarding = query(g, { kind: 'movement', units: [0] }).destinations.find(
    (d: any) => d.pirateCarrier != null,
  );
  assert.ok(boarding);
  let after = move(g, boarding.action);
  const ship = after.players[index].units.find((u: any) => u.pirate);
  assert.equal(ship.carried_units[0].id, 0);
  after = move(after, { Movement: 'Stop' });
  const routes = query(after, { kind: 'movement', units: [ship.id] }).destinations;
  assert.ok(routes.some((d: any) => d.position === 'D4'));
});
test('Hegemony uses a ship to found a city and lets its passengers disembark', async () => {
  const g = await carthageSea(),
    index = seat(g),
    p = g.players[index];
  p.units[0].carried_units = [{ id: 1, unit_type: 'Settler' }];
  p.next_unit_id = 2;
  g.map.tiles.find(([pos]: string[]) => pos === 'E3')[1] = 'Fertile';
  let after = move(
    g,
    view(g).specialActions.find((a: any) => a.id === 'Hegemony' || a.name === 'Hegemony').action,
  );
  assert.equal(after.board_history.frames.at(-1).title, 'Hegemony');
  assert.equal(
    JSON.parse(engine.stripSecret(serialize(after), 1 - index)).board_history.frames.at(-1).title,
    'Hegemony',
  );
  const publicGame = JSON.parse(engine.stripSecret(serialize(after), 1 - index));
  const frames = publicGame.board_history.frames;
  assert.equal(
    frameDetails(frames.at(-2), { ...frames.at(-1), title: 'Civilization ability' }, publicGame).caption,
    'Carthage · Hegemony',
    'existing recordings use the ability name from the public journal',
  );
  if (view(after).choiceDecision?.description.includes('new city'))
    after = move(after, { Response: { SelectPositions: ['E3'] } });
  assert.match((view(after).decision ?? view(after).choiceDecision).description, /passengers/);
  after = move(after, { Response: { SelectUnits: [1] } });
  assert.ok(after.players[index].cities.some((c: any) => c.position === 'E3'));
  assert.equal(after.players[index].units[0].position, 'E3');
  assert.ok(after.players[index].units[0].movement_restrictions.length);
  assert.equal(after.actions_left, g.actions_left - 1);
});
test('Dido can found a city at zero actions and Hanno can sail once for free', async () => {
  const g = await setup('Carthage', 'QueenDido'),
    index = seat(g),
    p = g.players[index];
  g.actions_left = 0;
  const destination = g.map.tiles.find(
    ([pos, t]: string[]) => t === 'Forest' && pos !== p.cities[0].position,
  )[0];
  for (const u of p.units) u.position = destination;
  const after = move(g, view(g).specialActions.find((a: any) => a.name === 'Founder').action);
  assert.equal(after.board_history.frames.at(-1).title, 'Founder');
  assert.equal(after.actions_left, 0);
  assert.equal(after.players[index].cities.length, 2);
  const sea = await carthageSea('Hanno'),
    sp = sea.players[seat(sea)];
  sea.players.find((p: any) => p.civilization === 'Pirates').units = [];
  let sailing = move(sea, view(sea).specialActions.find((a: any) => a.name === 'Navigator').action);
  assert.equal(sailing.board_history.frames.at(-1).title, 'Navigator');
  sailing = move(
    sailing,
    query(sailing, { kind: 'movement', units: [0] }).destinations.find((d: any) => d.position === 'D4')
      .action,
  );
  sailing = move(sailing, { Movement: 'Stop' });
  assert.equal(sailing.actions_left, sea.actions_left);
  assert.equal(query(sailing, { kind: 'movement', units: [0] }).destinations.length, 0);
});

test('Human Sacrifice explicitly exchanges chosen captives for mood and culture without an action', async () => {
  const g = await setup('Aztecs'),
    index = seat(g),
    p = g.players[index];
  addAdvance(p, 'Tactics', 'Myths', 'Rituals');
  heldCaptives(g, index, 3);
  g.actions_left = 0;
  let after = move(g, view(g).specialActions.find((a: any) => a.name === 'Human Sacrifice').action);
  after = move(after, { Response: { SelectCaptives: [p.captives[1], p.captives[2]] } });
  after = move(after, { Response: { ResourceReward: { mood_tokens: 1, culture_tokens: 1 } } });
  assert.equal(after.actions_left, 0);
  assert.deepEqual(after.players[index].captives, [p.captives[0]]);
  assert.equal(after.players[index].resources.mood_tokens, 9);
  assert.equal(after.players[index].resources.culture_tokens, 9);
  assert.equal(after.players[1 - index].held_units.infantry, 1);
});
test('Subterfuge sees the target before cancellation and Spy keeps the declared target', async () => {
  let g: any = await engine.init(3, [], { civilization: 'ChooseCivilization' }, 'subterfuge-three', {});
  for (const civ of ['Greece', 'Japan', 'Rome']) g = move(g, { ChooseCivilization: civ });
  g.current_player_index = 0;
  ['Greece', 'Japan', 'Rome'].forEach((c, i) => (g.players[i].civilization = c));
  g.players[0].action_cards = [7];
  g.players[0].resources.culture_tokens = 2;
  g.players[1].action_cards = [9];
  g.players[2].action_cards = [17];
  addAdvance(g.players[1], 'Tactics');
  g.players[1].units = [{ id: 0, unit_type: 'Infantry', position: g.players[0].cities[0].position }];
  let announced = move(g, { Playing: { ActionCard: 7 } });
  assert.equal(seat(announced), 0);
  assert.match((view(announced).decision ?? view(announced).choiceDecision).description, /player/);
  announced = move(announced, { Response: { SelectPlayer: 2 } });
  assert.equal(seat(announced), 1);
  assert.match(view(announced).decision.description, /targeting Rome/);
  assert.equal(announced.players[0].resources.culture_tokens, 2);
  const cancelled = move(announced, { Response: { SelectHandCards: [{ ActionCard: 9 }] } });
  assert.equal(cancelled.players[0].resources.culture_tokens, 2);
  assert.equal(cancelled.actions_left, g.actions_left);
  let accepted = move(announced, { Response: { SelectHandCards: [] } });
  if (view(accepted).decision?.fields.length)
    accepted = move(accepted, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(seat(accepted), 0);
  assert.equal(accepted.players[0].resources.culture_tokens, 1);
  assert.equal(accepted.events.at(-1).event_type.ActionCard.selected_player, 2);
  assert.match(JSON.stringify(view(accepted).decision), /City Development/);
});
test('Shogunate remains playable with zero actions and uses its free allowance automatically', async () => {
  const g = await setup('Japan'),
    p = g.players[seat(g)];
  addAdvance(p, 'Nationalism', 'Writing');
  p.action_cards = [17];
  p.cities.push({ position: 'C3', mood_state: 'Happy' });
  g.actions_left = 0;
  let after = move(g, { Playing: { ActionCard: 17 } });
  assert.equal(after.players[seat(g)].event_info['Shogunate card'], 'used');
  if (view(after).decision?.fields.length)
    after = move(after, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(after.actions_left, 1);
});
test('Loyalty lets the Celtic player direct Barbarian Tactical Retreat', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Celts', 'Viriatus');
  g.current_player_index = 1;
  const celts = g.players[0],
    attacker = g.players[1],
    barb = g.players.find((p: any) => p.civilization === 'Barbarians');
  celts.action_cards = [17];
  celts.units[0].position = 'E7';
  for (const t of g.map.tiles) t[1] = 'Fertile';
  attacker.cities = [{ position: 'D1', mood_state: 'Happy' }];
  attacker.units = [{ id: 0, position: 'E8', unit_type: 'Infantry' }];
  addAdvance(attacker, 'Tactics');
  barb.units = [{ id: 0, position: 'D8', unit_type: 'Infantry' }];
  barb.next_unit_id = 1;
  let after = move(g, { Movement: { Move: { units: [0], destination: 'D8', payment: {} } } });
  assert.equal(seat(after), 0);
  assert.match(view(after).decision.description, /Loyalty/);
  after = move(after, { Response: { SelectHandCards: [{ ActionCard: 17 }] } });
  assert.equal(seat(after), 0);
  const d = view(after).decision ?? view(after).choiceDecision;
  assert.match(d.description, /withdraw/);
  const target = d.options[0];
  after = move(after, query(after, { kind: 'decision', values: [target.value] }).action);
  assert.equal(after.players[barb.id].units[0].position, target.position);
  assert.equal(after.players[0].units[0].position, 'E7');
  assert.equal(after.players[0].action_cards?.length ?? 0, 0);
});
test('Nomads can reveal an actual unexplored block using the city alone', async () => {
  const g = await setup('Huns'),
    p = g.players[seat(g)],
    index = seat(g);
  addAdvance(p, 'Storage');
  p.units = [];
  let start: any, offer: any;
  for (const [pos, t] of g.map.tiles) {
    if (t === 'Unexplored') continue;
    p.cities[0].position = pos;
    const routes = query(g, { kind: 'movement', city: pos, units: [] }).destinations;
    offer = routes.find((r: any) => r.terrain === 'Unexplored');
    if (offer) {
      start = pos;
      break;
    }
  }
  assert.ok(offer, 'a home-region edge borders an unexplored map block');
  let after = move(g, offer.action);
  let d = view(after).explorationDecision;
  if (d) after = move(after, d.options?.[0]?.action ?? d.choices?.[0]?.action);
  assert.notEqual(after.map.tiles.find(([pos]: string[]) => pos === offer.position)[1], 'Unexplored');
  assert.ok(after.players[index].cities[0].position !== start || view(after).explorationDecision);
});
test('Hegemony rejects landing more than four armies on the new city', async () => {
  const g = await carthageSea(),
    p = g.players[seat(g)];
  g.map.tiles.find(([pos]: string[]) => pos === 'E3')[1] = 'Fertile';
  p.units[0].carried_units = [
    { id: 1, unit_type: 'Infantry' },
    { id: 2, unit_type: 'Infantry' },
  ];
  p.units.push(...[3, 4, 5].map((id) => ({ id, unit_type: 'Infantry', position: 'E3' })));
  p.next_unit_id = 6;
  let after = move(g, view(g).specialActions.find((a: any) => a.name === 'Hegemony').action);
  if ((view(after).decision ?? view(after).choiceDecision)?.kind === 'positions')
    after = move(after, { Response: { SelectPositions: ['E3'] } });
  const d = view(after).decision ?? view(after).choiceDecision;
  const values = d.options.map((o: any) => o.value);
  assert.equal(values.length, 2);
  assert.throws(() => query(after, { kind: 'decision', values }));
  after = move(after, query(after, { kind: 'decision', values: [values[0]] }).action);
  assert.equal(after.players[seat(g)].units.filter((u: any) => u.position === 'E3').length, 4);
});
test('Carthage can recruit a ship into its Port to attack pirates', async () => {
  const g = await carthageSea(),
    index = seat(g),
    p = g.players[index];
  p.units = [];
  p.next_unit_id = 0;
  g.players.find((p: any) => p.civilization === 'Pirates').units[0].position = 'D3';
  g.dice_roll_outcomes = [11, 0];
  const offered = query(g, {
    kind: 'recruit',
    city: 'D2',
    units: { ships: 1 },
    replaced: [],
    attackPirates: true,
  });
  let after = move(g, offered.action);
  assert.match(JSON.stringify(after.log), /Combat/);
  assert.equal(after.players[index].units.filter((u: any) => u.pirate).length, 0);
});
test('Storm Master restores a chosen non-leader unit lost to an event', async () => {
  const g = await setup('Aztecs', 'Marqzen'),
    p = g.players[seat(g)],
    index = seat(g),
    pos = p.cities[0].position;
  p.units.push({ id: p.next_unit_id++, position: pos, unit_type: 'Infantry' });
  p.incident_tokens = 1;
  g.incidents_left = [2, ...g.incidents_left.filter((id: number) => id !== 2)];
  let after = move(g, view(g).advances.find((a: any) => a.id === 'Storage').action);
  assert.match((view(after).decision ?? view(after).choiceDecision).description, /units to kill/);
  after = move(after, { Response: { SelectUnits: [0] } });
  assert.match(view(after).choiceDecision.name, /Storm Master/);
  after = move(after, { Response: { Bool: true } });
  assert.equal(after.players[index].units.filter((u: any) => u.unit_type === 'Settler').length, 1);
  assert.match(JSON.stringify(after.log), /Storm Master restored/);
});

test('Dido may sacrifice herself after a surviving round to force retreat without awarding victory points', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Rome');
  const defender = g.players[1];
  defender.civilization = 'Carthage';
  g.dice_roll_outcomes = Array(20).fill(2);
  g.players[0].units = [0, 1].map((id) => ({ id, position: 'E8', unit_type: 'Infantry' }));
  g.players[0].next_unit_id = 2;
  defender.units = [
    { id: 0, position: 'D8', unit_type: { Leader: 'QueenDido' } },
    { id: 1, position: 'D8', unit_type: 'Infantry' },
  ];
  defender.next_unit_id = 2;
  let after = move(g, { Movement: { Move: { units: [0, 1], destination: 'D8', payment: {} } } });
  if (seat(after) === 0) after = move(after, { Response: { Bool: false } });
  assert.equal(seat(after), 1);
  assert.match(view(after).choiceDecision.name, /Sacrifice/);
  after = move(after, { Response: { Bool: true } });
  assert.ok(after.players[1].units.every((u: any) => u.unit_type === 'Infantry'));
  assert.equal(after.players[1].units.length, 2);
  assert.ok(after.players[0].units.every((u: any) => u.position === 'E8'));
  assert.match(JSON.stringify(after.log), /gains no victory points for her sacrifice/);
});
test('Hannibal crosses a Mountain with Elephants and may leave it on the next Move action', async () => {
  const g = await fixture('civilizations/china/fast_war', 'Carthage', 'Hannibal');
  g.players[1].units = [];
  g.players[1].cities = [];
  const p = g.players[0];
  p.units.push({ id: 1, position: 'E8', unit_type: 'Elephant' });
  p.next_unit_id = 2;
  g.map.tiles.find(([pos]: string[]) => pos === 'D7')[1] = 'Mountain';
  let after = move(g, { Movement: { Move: { units: [0, 1], destination: 'D7', payment: {} } } });
  if (view(after).stopMovement) after = move(after, { Movement: 'Stop' });
  const destinations = query(after, { kind: 'movement', units: [0, 1] }).destinations;
  assert.ok(destinations.some((d: any) => d.position === 'E8'));
});
test('An allied pirate is removed before a colored ship in a naval battle', async () => {
  const g = await carthageSea(),
    index = seat(g),
    p = g.players[index],
    pirates = g.players.find((p: any) => p.civilization === 'Pirates');
  let after = move(
    g,
    query(g, { kind: 'movement', units: [0] }).destinations.find((d: any) => d.position === 'D4' && !d.attack)
      .action,
  );
  if (view(after).stopMovement) after = move(after, { Movement: 'Stop' });
  const ally = after.players[index].units.find((u: any) => u.pirate);
  after.players[1 - index].units = [{ id: 0, position: 'D5', unit_type: 'Ship' }];
  after.players[1 - index].next_unit_id = 1;
  after.dice_roll_outcomes = [0, 0, 11, 11, 11, 11];
  after = move(after, { Movement: { Move: { units: [0, ally.id], destination: 'D5', payment: {} } } });
  assert.ok(!after.players[index].units.some((u: any) => u.pirate));
  assert.ok(after.players[index].units.some((u: any) => u.id === 0));
  assert.equal(after.players[pirates.id].held_units?.ships ?? 0, 0);
});

test('all 27 new leaders load with full research under each government', async (t) => {
  const picker = view(
    await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'all-leader-abilities', {}),
  );
  for (const name of [
    'Aztecs',
    'Carthage',
    'Celts',
    'Egypt',
    'Huns',
    'Japan',
    'Maya',
    'Persia',
    'Phoenicia',
  ]) {
    const g = await setup(name),
      index = seat(g),
      p = g.players[index],
      before = view(g);
    const civilization = picker.civilizations.find((c: any) => c.name === name);
    const requirements = civilization.advances.map((a: any) =>
      /government/i.test(a.requirement)
        ? 'Voting'
        : before.advances.find((base: any) => base.name === a.requirement)?.id,
    );
    assert.ok(requirements.every(Boolean), `${name} unlock requirements are resolvable`);
    const governments = ['Democracy', 'Autocracy', 'Theocracy'];
    for (const government of governments) {
      p.advances = before.advances
        .filter((a: any) => !governments.includes(a.group) || a.group === government)
        .map((a: any) => a.id);
      for (const leader of before.cityActions[0].leaders) {
        const state = structuredClone(g),
          owner = state.players[index];
        owner.units.push({
          id: owner.next_unit_id++,
          position: owner.cities[0].position,
          unit_type: { Leader: leader.id },
        });
        await t.test(`${name} · ${government} · ${leader.name}`, () => {
          const visible = view(state, index);
          assert.equal(visible.players[index].leaders[0].name, leader.name);
          assert.equal(
            visible.players[index].civilizationAdvances.filter((a: any) => a.owned).length,
            4,
            `${name}: all civilization advances unlock`,
          );
        });
      }
    }
  }
});
