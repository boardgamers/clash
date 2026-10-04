import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'turn-warning-npcs', {})).players.slice(2);
function fixture(name: string) {
  const g = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!g.players.some((p: any) => p.civilization === npc.civilization))
      g.players.push({ ...npc, id: g.players.length });
  return g;
}
const view = (g: any, seat = g.current_player_index): View =>
  JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), seat), seat));
const move = (g: any, action: unknown) =>
  JSON.parse(
    engine.tryMove(JSON.stringify(g), JSON.stringify(action), engine.currentPlayer(JSON.stringify(g))),
  );
function trading(currency = false) {
  const g = fixture(`advances/trade_routes${currency ? '_with_currency' : ''}`);
  g.current_player_index = 1;
  g.round = 2;
  g.actions_left = 0;
  g.players[1].resources.food = g.players[1].resource_limit.food;
  return g;
}

test('End turn warns about actual routes overflowing food even with no actions left', () => {
  const g = trading();
  const before = JSON.stringify(g);
  const v = view(g);
  assert.equal(v.canEndTurn, true);
  assert.deepEqual(v.endTurnTradeWarning, { routes: 2, waste: { food: 2 } });
  assert.equal(view(g, 0).endTurnTradeWarning, null);
  assert.equal(JSON.stringify(g), before, 'Forecast must not mutate the board');
  assert.deepEqual(view(g).endTurnTradeWarning, v.endTurnTradeWarning);
  g.players[1].resources.food = 1;
  assert.deepEqual(view(g).endTurnTradeWarning?.waste, { food: 1 });
  g.players[1].resources.food = 0;
  assert.equal(view(g).endTurnTradeWarning, null);
});

test('Currency warns only when food/gold choices cannot fit, including mixed rewards', () => {
  const g = trading(true);
  const p = g.players[1];
  assert.equal(view(g).endTurnTradeWarning, null, 'Gold can fit when food is full');
  p.resources.gold = p.resource_limit.gold;
  assert.equal(view(g).endTurnTradeWarning?.routes, 2);
  assert.equal(
    Object.values(view(g).endTurnTradeWarning!.waste).reduce((a, b) => a + b, 0),
    2,
  );
  p.resources.food -= 1;
  p.resources.gold -= 1;
  assert.equal(view(g).endTurnTradeWarning, null, 'One food plus one gold fits');
  p.resources.gold += 1;
  assert.equal(
    Object.values(view(g).endTurnTradeWarning!.waste).reduce((a, b) => a + b, 0),
    1,
  );
});

test('no warning without usable routes, with an expired borrowed advance, or after the final turn', () => {
  const g = trading();
  g.players[0].cities.forEach((c: any) => (c.mood_state = 'Angry'));
  assert.equal(view(g).endTurnTradeWarning, null);
  const borrowed = trading();
  borrowed.players[1].advances = borrowed.players[1].advances.filter((a: string) => a !== 'TradeRoutes');
  borrowed.players[1].great_library_advance = 'TradeRoutes';
  assert.equal(view(borrowed).endTurnTradeWarning, null);
  const borrowedCurrency = trading();
  borrowedCurrency.players[1].great_library_advance = 'Currency';
  assert.ok(view(borrowedCurrency).endTurnTradeWarning, 'Borrowed Currency expires before the next turn');
  for (const [length, age] of [
    ['Standard', 6],
    ['Epic', 10],
  ] as const) {
    const last = trading();
    last.options = { ...last.options, length };
    last.age = age;
    last.round = 3;
    assert.equal(view(last).endTurnTradeWarning, null);
    last.round = 2;
    assert.ok(view(last).endTurnTradeWarning);
  }
});

test('Prosperity can use uncapped mood tokens instead of overflowing resources', () => {
  const g = trading();
  const p = g.players[1];
  p.civilization = 'India';
  p.advances = [...new Set([...p.advances, 'Currency'])];
  p.resources.gold = 7;
  assert.equal(view(g).endTurnTradeWarning, null);
});

test('research warns when a selectable advance uses the last event marker', () => {
  const g = fixture('advances/writing');
  for (const remaining of [3, 2, 1]) {
    g.players[0].incident_tokens = remaining;
    const v = view(g);
    assert.ok(v.advances.some((a) => a.action));
    for (const a of v.advances) assert.equal(a.triggersEvent, remaining === 1 && !!a.action, a.name);
    assert.ok(view(g, 1).advances.every((a) => !a.triggersEvent));
  }
  const farming = view(g).advances.find((a) => a.id === 'Farming')!;
  assert.ok(farming.action);
  const after = move(g, farming.action);
  assert.equal(after.players[0].incident_tokens, 3);
});

test('unaffordable research still explains that it would use the last event marker', () => {
  const g = fixture('advances/writing');
  g.players[0].resources = {};
  g.players[0].incident_tokens = 1;
  const advances = view(g).advances;
  const unaffordable = advances.filter((a) => a.reason === 'Not enough resources');
  assert.ok(unaffordable.length);
  for (const advance of unaffordable) {
    assert.equal(advance.action, null, 'the warning never enables research');
    assert.equal(advance.triggersEvent, true, advance.name);
  }
  assert.ok(advances.filter((a) => a.owned).every((a) => !a.triggersEvent));
  g.players[0].incident_tokens = 2;
  assert.ok(view(g).advances.every((a) => !a.triggersEvent));
});

test('card advances and Great Library do not warn even with one marker left', () => {
  for (const [name, action] of [
    ['action_cards/advance', { Playing: { ActionCard: 2 } }],
    ['action_cards/synergies', { Playing: { ActionCard: 34 } }],
  ] as const) {
    const g = fixture(name);
    g.players[0].incident_tokens = 1;
    let pending = move(g, action);
    if (view(pending).decision?.fields.length)
      pending = move(pending, { Response: { Payment: [{ culture_tokens: 1 }] } });
    assert.ok(view(pending).advances.some((a) => a.action));
    assert.ok(view(pending).advances.every((a) => !a.triggersEvent));
  }
  let library = fixture('wonders/library');
  library.players[0].incident_tokens = 1;
  library = move(library, view(library).specialActions!.find((a) => a.name === 'Great Library')!.action);
  assert.ok(view(library).advances.some((a) => a.action));
  assert.ok(view(library).advances.every((a) => !a.triggersEvent));
});

test('the end-of-age free advance still warns because it consumes a marker', () => {
  let g = fixture('status_phase/free_advance');
  g.players[1].incident_tokens = 1;
  g = move(g, { Playing: 'EndTurn' });
  const v = view(g, engine.currentPlayer(JSON.stringify(g)));
  assert.equal(v.decision?.advanceMode, 'free');
  assert.ok(v.advances.some((a) => a.action));
  assert.ok(v.advances.filter((a) => a.action).every((a) => a.triggersEvent));
});

test('a leader research action warns even though it costs no normal action', async () => {
  let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'research-leader-warning', {});
  let g = JSON.parse(
    engine.tryMove(raw, JSON.stringify({ ChooseCivilization: 'India' }), engine.currentPlayer(raw)),
  );
  g = move(g, { ChooseCivilization: 'Rome' });
  const p = g.players[g.current_player_index];
  p.resources = { food: 7, wood: 7, ore: 7, ideas: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  p.resource_limit.food = 7;
  p.incident_tokens = 1;
  p.units.push({ id: p.next_unit_id++, position: p.cities[0].position, unit_type: { Leader: 'SriGupta' } });
  g = move(g, view(g).specialActions!.find((a) => a.name === 'Golden Age')!.action);
  if (view(g).decision?.fields.length) g = move(g, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(view(g).decision?.advanceMode, 'paid');
  assert.ok(view(g).advances.some((a) => a.action));
  assert.ok(
    view(g)
      .advances.filter((a) => a.action)
      .every((a) => a.triggersEvent),
  );
});

test('all eight wonders are referenceable independently of private hands and deck order', () => {
  const g = trading();
  const catalog = view(g).wonderCatalog!;
  assert.equal(catalog.length, 8);
  assert.ok(
    catalog.every((w) => w.id !== 'Hidden' && !w.action && w.requiredAdvance && Object.keys(w.cost).length),
  );
  assert.ok(catalog.some((w) => w.name === 'Great Arena'));
  assert.ok(catalog.some((w) => w.name === 'Great Pyramid'));
  assert.deepEqual(catalog, view(g, 0).wonderCatalog);
  g.players[0].wonder_cards = ['Colosseum'];
  g.players[1].wonder_cards = ['GreatWall'];
  g.wonders_left = ['Pyramids', 'GreatGardens'];
  assert.deepEqual(catalog, view(g).wonderCatalog);
  assert.deepEqual(catalog, view(g, 0).wonderCatalog);
});
