// Explicit maintenance tool, never run by tests. Requires the published 0.4.160 package.
// node scripts/generate-legacy-ballcourts-history.mjs /absolute/path/to/package/server.js
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import assert from 'node:assert/strict';
const path = resolve(process.argv[2]);
assert.equal(JSON.parse(readFileSync(resolve(dirname(path), 'package.json'))).version, '0.4.160');
const engine = createRequire(import.meta.url)(path);
let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'ballcourts-history', {});
const seat = () => engine.currentPlayer(raw);
const view = () => JSON.parse(engine.webView(engine.stripSecret(raw, seat()), seat()));
const move = (action) => { raw = engine.tryMove(raw, JSON.stringify(action), seat()); };
const collect = (bonus = false) => {
  const city = view().cities[0];
  const selections = bonus
    ? city.choices.slice(0, 2).map(c => ({ position: c.position, pile: c.pile, times: 1 }))
    : [{ position: city.position, pile: { food: 1 }, times: 1 }];
  return JSON.parse(engine.webQuery(engine.stripSecret(raw, seat()), seat(), JSON.stringify({
    kind: 'collect', city: city.position, variant: 'Collect', ballcourts: bonus, selections,
  }))).action;
};
move({ ChooseCivilization: 'Maya' });
move({ ChooseCivilization: 'Rome' });
move(view().advances.find(a => a.id === 'Arts').action);
move(collect());
move(collect());
move({ Playing: 'EndTurn' });
move({ Playing: 'EndTurn' });
move(view().advances.find(a => a.id === 'Sports').action);
const before = engine.logLength(raw);
const bonus = collect(true);
move(bonus);
const paid = raw;
move(collect());
const full = raw;
const fields = ['players', 'events', 'actions_left', 'rng', 'time_increments', 'state'];
const projection = raw => Object.fromEntries(fields.map(k => [k, JSON.parse(raw)[k]]).filter(([, v]) => v !== undefined));
for (const save of [paid, full]) assert.deepEqual(projection(engine.replay(save, {})), projection(save));
const data = JSON.parse(full);
const fixture = Object.fromEntries(['seed', 'options', 'players', 'log', 'log_index', 'time_increments'].map(k => [k, data[k]]));
fixture.players = fixture.players.map(({ id, name, civilization, settings }) => ({ id, name, civilization, settings }));
const root = new URL('../server/tests/fixtures/', import.meta.url);
writeFileSync(new URL('legacy_ballcourts_history.json', root), JSON.stringify(fixture, null, 2) + '\n');
writeFileSync(new URL('legacy_ballcourts_history.expected.json', root), JSON.stringify({
  before, afterPayment: engine.logLength(paid), full: engine.logLength(full), bonus,
  paid: projection(paid), following: projection(full),
}, null, 2) + '\n');
