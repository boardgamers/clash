// Run after building the engine bridge:
//   node scripts/wonder-payment-perf.mjs
// Optional slow-machine override: WONDER_PAYMENT_BUDGET_MS=500 node ...
// Uses local fixtures only: no network calls or live-game moves.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { performance } from 'node:perf_hooks';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const original = JSON.parse(await readFile(new URL('../../server/tests/test_games/wonders/colosseum.json', import.meta.url), 'utf8'));
const npcs = JSON.parse(await engine.init(2, [], {}, 'wonder-payment-perf', {})).players.slice(2);
const budget = Number(process.env.WONDER_PAYMENT_BUDGET_MS ?? 250);
const construct = JSON.stringify({ Playing: { WonderCard: 'Colosseum' } });
const cases = [
  { name: 'ordinary gold', civilization: 'Greece', stock: { food: 3, wood: 4, ore: 5, gold: 2, culture_tokens: 5 }, choices: 10 },
  { name: 'Architecture, no mood', civilization: 'Egypt', stock: { food: 3, wood: 4, ore: 5, gold: 2, culture_tokens: 5 }, choices: 10 },
  { name: 'Architecture, late game', civilization: 'Egypt', stock: { food: 7, wood: 7, ore: 7, ideas: 5, gold: 7, mood_tokens: 7, culture_tokens: 7 }, choices: null },
];
let slow = false;
for (const fixture of cases) {
  const game = structuredClone(original);
  game.players.push(...structuredClone(npcs));
  game.players[0].civilization = fixture.civilization;
  game.players[0].resources = fixture.stock;
  const raw = engine.stripSecret(JSON.stringify(game), 0);
  const timings = {};
  const time = (name, fn) => {
    const start = performance.now();
    const value = fn();
    timings[name] = Number((performance.now() - start).toFixed(2));
    return value;
  };
  const before = JSON.parse(time('availability', () => engine.webView(raw, 0)));
  assert(before.wonderCards[0].action, 'The wonder is constructible');
  const after = time('construct', () => engine.tryMove(raw, construct, 0));
  const view = JSON.parse(time('paymentView', () => engine.webView(after, 0)));
  const field = view.decision.fields[0];
  assert.equal(field.name, 'Pay to build wonder');
  assert.equal(field.choices?.length ?? null, fixture.choices);
  const query = JSON.parse(time('paymentQuery', () => engine.webQuery(after, 0, JSON.stringify({ kind: 'decision', values: [], payments: [field.initial] }))));
  assert(query.action, 'The initial payment is valid before spending');
  time('cachedStateView', () => engine.webView(after, 0));
  assert.deepEqual(JSON.parse(after).players[0].resources, fixture.stock, 'Opening the payment has not spent resources');
  if (fixture.choices === null) {
    // Large spaces must still allow legal gold + mood + culture mixes through
    // direct selectors; the display-list limit must never become a rules limit.
    const payment = { food: 1, wood: 2, ore: 3, gold: 4, mood_tokens: 4, culture_tokens: 3 };
    const selected = JSON.parse(time('mixedQuery', () => engine.webQuery(after, 0, JSON.stringify({ kind: 'decision', values: [], payments: [payment] }))));
    assert(selected.action);
    const paid = JSON.parse(engine.tryMove(after, JSON.stringify(selected.action), 0));
    assert(paid.players[0].cities.some((city) => city.city_pieces?.wonders?.includes('Colosseum')));
    for (const [resource, amount] of Object.entries(payment)) assert.equal(paid.players[0].resources[resource] ?? 0, fixture.stock[resource] - amount);
  }
  console.log(`${fixture.name}: ${JSON.stringify(timings)} ms`);
  slow ||= timings.paymentView > budget || timings.cachedStateView > budget;
}
assert(!slow, `Wonder payment view exceeded ${budget} ms; investigate before raising the budget`);
