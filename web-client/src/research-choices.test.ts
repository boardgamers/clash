import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { researchDecision } from './decision-controls.ts';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'research-npcs', {})).players.slice(2);
const fixture = (name: string) => {
  const raw = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!raw.players.some((p: any) => p.civilization === npc.civilization))
      raw.players.push({ ...npc, id: raw.players.length });
  return raw;
};
const serialize = (g: any) => (typeof g === 'string' ? g : JSON.stringify(g));
const view = (g: any, seat = engine.currentPlayer(serialize(g))): View =>
  JSON.parse(engine.webView(engine.stripSecret(serialize(g), seat), seat));
const move = (g: any, action: unknown) =>
  JSON.parse(engine.tryMove(serialize(g), JSON.stringify(action), engine.currentPlayer(serialize(g))));
function exactChoices(v: View) {
  assert.ok(researchDecision(v));
  assert.deepEqual(
    v.advances
      .filter((a) => a.action)
      .map((a) => a.id)
      .sort(),
    v.decision!.options.map((o) => o.value).sort(),
  );
  assert.ok(
    v.advances.every((a) => a.payments.length === 0),
    'Pending choices must never submit an ordinary research action',
  );
}

test('Synergies uses the research tree for both paid choices, with the second restricted to its category', () => {
  const initial = fixture('action_cards/synergies');
  let g = move(initial, { Playing: { ActionCard: 34 } });
  for (const [id, stage] of [
    ['Fishing', 'first'],
    ['WarShips', 'second'],
  ]) {
    const v = view(g);
    exactChoices(v);
    assert.equal(v.decision!.advanceMode, 'paid');
    assert.match(v.decision!.description, new RegExp(stage));
    const selected = v.advances.find((a) => a.id === id)!;
    assert.equal(selected.costAmount, 2);
    assert.deepEqual(selected.action, { Response: { SelectAdvance: id } });
    assert.ok(!researchDecision(view(g, 1)));
    if (stage === 'second')
      assert.ok(v.advances.filter((a) => a.action).every((a) => a.group === 'Seafaring'));
    g = move(g, selected.action);
    assert.equal(view(g).decision!.fields[0].name, `Pay for ${selected.name}`);
    g = move(g, { Response: { Payment: [{ ideas: 2 }] } });
  }
  assert.ok(!researchDecision(view(g)));
  assert.equal(g.actions_left, initial.actions_left - 1);
  assert.equal(g.players[0].resources.ideas, initial.players[0].resources.ideas - 4);
  assert.equal(g.players[0].event_tokens, initial.players[0].event_tokens);
});

test('Philosophy adds a sourced idea bonus to Science cards and it matches the awarded resources', () => {
  const g = fixture('advances/writing');
  const p = g.players[0];
  p.advances = [...new Set([...(p.advances ?? []), 'Writing', 'Philosophy'])];
  p.advances = p.advances.filter((a: string) => !['Math', 'Astronomy', 'Medicine', 'Metallurgy'].includes(a));
  p.resources.ideas = 1;
  const science = view(g).advances.filter((a) => a.group === 'Science');
  assert.equal(science.length, 4);
  for (const a of science) {
    assert.deepEqual(a.bonusEffects, [{ source: 'Philosophy', pile: { ideas: 1 } }]);
    assert.deepEqual(a.bonus, { culture_tokens: 1 });
  }
  assert.ok(
    view(g)
      .advances.filter((a) => a.group !== 'Science')
      .every((a) => !a.bonusEffects?.length),
  );
  const math = science.find((a) => a.id === 'Math')!;
  const after = move(g, math.payments.find((option) => option.payment.food === 2)!.action);
  assert.equal(after.players[0].resources.ideas, 2);
  assert.equal(after.players[0].resources.culture_tokens, p.resources.culture_tokens + 1);
  p.advances = p.advances.filter((a: string) => a !== 'Philosophy');
  assert.ok(view(g).advances.every((a) => !a.bonusEffects?.length));
});

test('Great Library uses the shared tree without promising research bonuses or applying prerequisites', () => {
  let g = fixture('wonders/library');
  const ability = view(g).specialActions!.find((a) => a.name === 'Great Library')!;
  g = move(g, ability.action);
  const v = view(g);
  exactChoices(v);
  assert.equal(v.decision!.advanceMode, 'borrow');
  assert.ok(v.advances.every((a) => a.bonus === null && !a.bonusEffects?.length));
  const engineering = v.advances.find((a) => a.id === 'Engineering')!;
  assert.equal(engineering.costAmount, 0);
  const before = g.players[0];
  g = move(g, engineering.action);
  assert.equal(g.players[0].great_library_advance, 'Engineering');
  assert.deepEqual(g.players[0].resources, before.resources);
  assert.deepEqual(g.players[0].advances, before.advances);
});

test('Free Economy exposes its mood cost before collect is submitted', () => {
  const v = view(fixture('advances/collect_free_economy'));
  const economy = v.collectActions!.find((a) => a.name === 'Free Economy')!;
  assert.equal(economy.free, true);
  assert.deepEqual(economy.payment, { mood_tokens: 1 });
  assert.deepEqual(v.collectActions!.find((a) => a.value === 'Collect')!.payment, {});
});

test('old unrestricted saves stop at the migration boundary and all future information reveals lock undo', () => {
  let g = fixture('advances/collect_free_economy');
  const action = {
    Playing: {
      Collect: {
        city_position: 'C2',
        collections: [{ position: 'B1', pile: { ore: 1 }, times: 1 }],
        action_type: 'Collect',
      },
    },
  };
  g = move(g, action);
  assert.equal(view(g).canUndo, true);
  g.options = { undo: 'SamePlayer' };
  assert.equal(view(g).canUndo, false);
  assert.throws(() => move(g, 'Undo'), /undone/);
  g = move(g, action);
  assert.equal(g.options?.undo, undefined);
  assert.equal(view(g).canUndo, true, 'New safe moves retain normal undo');
  g = move(g, 'Undo');
  assert.equal(view(g).canUndo, false, 'Cannot pass the old unprotected boundary');
  for (const setting of ['SamePlayer', 'ProtectSecrets', undefined]) {
    const writing = fixture('advances/writing');
    writing.options = { undo: setting };
    const revealed = move(writing, {
      Playing: { Advance: { advance: 'Writing', payment: { food: 1, gold: 1 } } },
    });
    assert.equal(view(revealed).canUndo, false, `Card draw with ${setting}`);
    assert.throws(() => move(revealed, 'Undo'), /undone/);
  }
});
