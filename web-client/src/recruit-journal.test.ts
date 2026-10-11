import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { journal } from './journal.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'recruit-journal', {})).players.slice(2);
const fixture = () => {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/advances/sanitation_and_draft.json', import.meta.url), 'utf8'));
  for (const npc of npcs) if (!game.players.some((p: any) => p.civilization === npc.civilization)) game.players.push({ ...npc, id: game.players.length });
  game.players[0].resources = { food: 7, ore: 7, gold: 7, mood_tokens: 7, culture_tokens: 7 };
  return game;
};
for (const [name, units, payment, expected] of [
  ['normal infantry', { infantry: 1 }, { food: 1, ore: 1 }, []],
  ['normal settler', { settlers: 1 }, { food: 2 }, []],
  ['Draft', { infantry: 1 }, { mood_tokens: 1 }, ['Draft']],
  ['Sanitation', { settlers: 1 }, { mood_tokens: 1 }, ['Sanitation']],
  ['both substitutions', { infantry: 1, settlers: 1 }, { mood_tokens: 2 }, ['Draft', 'Sanitation']],
  ['ambiguous substitution', { infantry: 1, settlers: 1 }, { mood_tokens: 1, gold: 2 }, ['Sanitation']],
] as const) test(`recruit journal records only paid substitutions: ${name}`, () => {
  const game = fixture();
  const action = { Playing: { Recruit: { units, city_position: 'A1', payment } } };
  const after = JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), 0));
  const actions = after.log.flatMap((a: any) => a.rounds.flatMap((r: any) => r.turns.flatMap((t: any) => t.actions)));
  const last = actions.at(-1);
  const used = last.items.filter((i: any) => i.Text && ['Draft', 'Sanitation'].includes(i.origin?.Advance)).map((i: any) => i.origin.Advance).sort();
  assert.deepEqual(used, [...expected].sort());
  const entries = journal(after).filter(e => e.title === 'Recruit');
  assert.equal(entries.length, 1, 'Recruitment and payment share one row');
  assert(!journal(after).some(e => ['Draft', 'Sanitation'].includes(e.title)));
});

test('historical optional-discount announcements disappear without altering saved records or costs', () => {
  const game = JSON.parse(readFileSync(new URL('../../server/tests/test_games/advances/sanitation_and_draft.outcome.json', import.meta.url), 'utf8'));
  const items = game.log[0].rounds[0].turns[0].actions[0].items;
  items.push({ player: 0, origin: { Advance: 'Draft' }, Text: 'Reduce the cost of 1 Infantry to 1 mood token' });
  for (const item of items) if (item.Text === 'Used Sanitation for 1 Settler') item.Text = 'Reduce the cost of 1 Settler to 1 mood token';
  const before = JSON.stringify(game);
  const rows = journal(game);
  assert(!rows.some(row => ['Draft', 'Sanitation'].includes(row.title)));
  assert.equal(rows.filter(row => row.title === 'Recruit').length, 1);
  assert(rows.some(row => row.tokens.some(t => t.icon === 'mood_tokens' && t.value === '−1')));
  assert.equal(JSON.stringify(game), before);
});

for (const [civilization, payment, used] of [
  ['Greece', { food: 1, ore: 1 }, []],
  ['Greece', { mood_tokens: 1 }, ['Draft']],
  ['Greece', { culture_tokens: 1 }, ['Draft', 'Sparta']],
  ['Carthage', { food: 1, ore: 1 }, []],
  ['Carthage', { mood_tokens: 1 }, ['Draft']],
  ['Carthage', { gold: 1 }, ['Draft', 'Mercenaries']],
] as const) test(`recruitment attributes civilization conversions only when needed: ${civilization} ${JSON.stringify(payment)}`, () => {
  const game = fixture(); game.players[0].civilization = civilization;
  const after = JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify({ Playing: { Recruit: { units: { infantry: 1 }, city_position: 'A1', payment } } }), 0));
  const actions = after.log.flatMap((a: any) => a.rounds.flatMap((r: any) => r.turns.flatMap((t: any) => t.actions)));
  const actual = actions.at(-1).items.filter((i: any) => i.Text && /Used Draft|Paid Draft|Used Mercenaries/.test(i.Text)).map((i: any) => i.origin.Advance ?? i.origin.SpecialAdvance).sort();
  assert.deepEqual(actual, [...used].sort());
});
