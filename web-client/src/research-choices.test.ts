import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { researchDecision } from './decision-controls.ts';
import { researchFreeHints } from './research-links.ts';
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

test('research hints identify future resource-free unlocks and disappear once their sources are owned', () => {
  const game = fixture('advances/writing');
  const p = game.players[0];
  p.advances = ['Farming', 'Mining', 'Fishing'];
  const before = view(game).advances;
  const hint = (id: string) =>
    researchFreeHints(
      before.find((a) => a.id === id)!,
      before,
    );
  for (const id of ['Engineering', 'Roads']) assert.equal(hint(id)[0].name, 'Math');
  for (const id of ['Navigation', 'Cartography']) assert.equal(hint(id)[0].name, 'Astronomy');
  for (const a of before.filter((a) => a.group === 'Science')) {
    assert.deepEqual(researchFreeHints(a, before), [
      { id: 'Priesthood', name: 'Priesthood', oncePerTurn: true },
    ]);
  }
  assert.deepEqual(hint('Storage'), []);
  p.advances.push('Math', 'Astronomy', 'Myths', 'Priesthood');
  const after = view(game).advances;
  for (const id of ['Engineering', 'Roads', 'Navigation', 'Cartography', 'Medicine', 'Metallurgy']) {
    assert.deepEqual(
      researchFreeHints(
        after.find((a) => a.id === id)!,
        after,
      ),
      [],
    );
  }
});
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

test('Great Explorer grants a free Seafaring advance even when the entire map is explored', () => {
  const initial = fixture('advances/writing');
  const p = initial.players[0];
  p.action_cards = [118];
  p.advances = ['Farming', 'Mining', 'Fishing', 'Navigation'];
  initial.map.tiles = initial.map.tiles.map(([position, terrain]: string[]) => [
    position,
    terrain === 'Unexplored' ? 'Fertile' : terrain,
  ]);
  const offer = view(initial, 0).actionCards!.find((a) => a.id === 118)!;
  assert.ok(offer.action);
  let g = move(initial, offer.action);
  const decision = view(g, 0);
  exactChoices(decision);
  assert.deepEqual(decision.decision!.options.map((o) => o.value).sort(), ['Cartography', 'WarShips']);
  g = move(g, { Response: { SelectAdvance: 'WarShips' } });
  assert.ok(g.players[0].advances.includes('WarShips'));
  assert.equal(g.actions_left, initial.actions_left - 1);
  assert.deepEqual(g.players[0].resources, p.resources);
  assert.equal(g.players[0].event_tokens, p.event_tokens);
  assert.ok(!(g.players[0].action_cards ?? []).includes(118));
  assert.equal(view(g, 0).decision, null);
});

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

test('Teach Us highlights only the defeated player’s advances without an event warning', () => {
  let game = fixture('action_cards/teach_us');
  game.players[1].advances.push('Fishing');
  game = move(game, { Movement: { Move: { units: [0, 1, 2, 3, 4, 5], destination: 'C1', payment: {} } } });
  game = move(game, { Response: { SelectHandCards: [{ ActionCard: 35 }] } });
  const seat = engine.currentPlayer(serialize(game));
  game.players[seat].incident_tokens = 1;
  const v = view(game);
  exactChoices(v);
  assert.equal(v.decision!.name, 'Teach us');
  assert.ok(v.advances.some((a) => a.action));
  assert.ok(v.advances.every((a) => !a.triggersEvent));
  const advance = v.advances.find((a) => a.action)!;
  const result = move(game, advance.action);
  assert.equal(result.players[seat].incident_tokens, 1);
  assert.ok(result.players[seat].advances.includes(advance.id));
});
