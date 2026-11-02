import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { ActionQueue } from './action-queue.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
function fixture() {
  const g=JSON.parse(readFileSync(new URL('../scripts/async-preview-fixture.json',import.meta.url),'utf8'));
  g.events=[];g.state='Playing';g.actions_left=3;
  g.board_history={id:'queue-test',frames:[]};
  g.players[0].resources.food=2;
  for(const c of g.players[0].cities){c.activations=0;c.mood_state='Happy';}
  return JSON.stringify(g);
}
const collect={Playing:{Collect:{city_position:'A1',collections:[{position:'A1',pile:{food:1},times:1}],action_type:'Collect'}}};
test('projects without mutating the base, then replays on the resolved board',()=>{
  const base=fixture(),q=new ActionQueue(engine,0,base);
  assert.equal(q.append(collect).ok,true);
  assert.equal(JSON.parse(base).players[0].resources.food,2);
  assert.equal(JSON.parse(q.projected).players[0].resources.food,3);
  const authoritative=JSON.parse(base);
  authoritative.events=JSON.parse(readFileSync(new URL('../scripts/async-preview-fixture.json',import.meta.url),'utf8')).events;
  const resolved=engine.tryMove(JSON.stringify(authoritative),JSON.stringify({Response:{SelectPositions:['B2']}}),1);
  const replay=q.replay(resolved);
  assert.equal(replay.applied,1);
  assert.equal(replay.remaining.length,0);
  assert.equal(JSON.parse(replay.raw).players[0].resources.food,3);
  assert.deepEqual(JSON.parse(replay.raw).players[1].units,JSON.parse(resolved).players[1].units);
});
test('invalid replay stops and retains its suffix, without ending the turn',()=>{
  const base=fixture(),q=new ActionQueue(engine,0,base);
  assert.equal(q.append(collect).ok,true);
  assert.equal(q.append({Playing:'EndTurn'}).ok,true);
  const changed=JSON.parse(base);changed.actions_left=0;
  const result=q.replay(JSON.stringify(changed));
  assert.equal(result.applied,0);assert.equal(result.remaining.length,2);
  assert.equal(engine.currentPlayer(result.raw),0);
});
test('end turn is terminal, undoable and executes only during replay',()=>{
  const base=fixture(),q=new ActionQueue(engine,0,base);
  assert.equal(q.append({Playing:'EndTurn'}).ok,true);
  assert.equal(q.append(collect).ok,false);
  assert.equal(engine.currentPlayer(q.projected),0);
  q.undo();assert.equal(q.ended,false);
  assert.equal(q.append(collect).ok,true);
  assert.equal(q.append({Playing:'EndTurn'}).ok,true);
  const result=q.replay(base);assert.equal(result.applied,2);
  assert.equal(engine.currentPlayer(result.raw),1);
});
test('invalid actions never enter the queue',()=>{
  const q=new ActionQueue(engine,0,fixture());
  assert.equal(q.append({Playing:{Collect:{...collect.Playing.Collect,collections:[{position:'Z9',pile:{food:1},times:1}]}}}).ok,false);
  assert.equal(q.entries.length,0);
});

for (const convert of [false, true]) test(`Egypt's five-tile collection queues and replays with Metallurgy ${convert ? 'accepted' : 'declined'}`, () => {
  // Public Play-clash-38047cc1da snapshot: names removed and history reset.
  // D2 is size 4 / Happy / capacity 5 with Public Education, Flood Plains,
  // Husbandry and Metallurgy. Greece is still placing its replacement settler.
  const raw = readFileSync(new URL('../scripts/egypt-collect-queue-fixture.json', import.meta.url), 'utf8');
  const plan = JSON.parse(engine.webPlanning(raw, 0));
  assert.equal(plan.label, 'Placing a settler');
  const q = new ActionQueue(engine, 0, plan.base);
  const stop = { Movement: 'Stop' };
  assert.deepEqual(q.append(stop), { ok: true });
  const city = JSON.parse(engine.webView(q.projected, 0)).cities.find((c: any) => c.position === 'D2');
  assert.deepEqual([city.size, city.mood, city.capacity], [4, 'Happy', 5]);
  const selections = [
    ['D2', 'food'], ['B3', 'ore'], ['E2', 'ore'], ['C2', 'wood'], ['D1', 'wood'],
  ].map(([position, resource]) => ({ position, pile: { [resource]: 1 }, times: 1 }));
  const preview = JSON.parse(engine.webCollectPreview(q.projected, 0, 'D2', JSON.stringify(selections)));
  assert.deepEqual(preview.total, { food: 1, ideas: 1, ore: 2, wood: 2 });
  assert.ok(preview.effects.some((effect: any) => effect.source === 'Public Education' && effect.description === 'Gain 1 idea'));
  assert.deepEqual(q.append(preview.action), { ok: true });
  const collected = JSON.parse(q.projected);
  assert.equal(collected.events[0].handler.origin.Advance, 'Metallurgy');
  // The follow-up retains unordered alternatives, not selected-resource order.
  assert.deepEqual(new Set(collected.events[0].event_type.Collect.choices.C2.map((pile: any) => JSON.stringify(pile))),
    new Set(['{"food":1}', '{"wood":1}']));
  const response = { Response: { Bool: convert } };
  assert.deepEqual(q.append(response), { ok: true });
  assert.equal(q.count, 2, 'Metallurgy stays in the collection undo group');
  const expected = { food: 5, wood: 2, ore: convert ? 1 : 2, ideas: 2, gold: convert ? 4 : 3, mood_tokens: 1, culture_tokens: 3 };
  assert.deepEqual(JSON.parse(q.projected).players[0].resources, expected);
  const actions = q.entries.map(entry => entry.move);
  q.undo();
  assert.equal(q.count, 1);
  assert.deepEqual(JSON.parse(q.projected).players[0].resources, JSON.parse(raw).players[0].resources);
  assert.deepEqual(q.append(preview.action), { ok: true });
  assert.deepEqual(q.append(response), { ok: true });

  const settle = JSON.stringify({ Response: { SelectPositions: ['B6'] } });
  const resolved = engine.tryMove(raw, settle, 1);
  const replay = q.replay(resolved);
  assert.equal(replay.applied, 3, replay.error);
  assert.deepEqual(replay.remaining, []);
  assert.deepEqual(JSON.parse(replay.raw).players[0].resources, expected);
  assert.deepEqual(JSON.parse(replay.raw).players[1].units, JSON.parse(resolved).players[1].units);

  // Exercise the authoritative safe_step guard and persisted server replay too.
  const saved = engine.tryMove(raw, JSON.stringify({ QueueMoves: { key: plan.key, actions, waiting: false } }), 0);
  assert.deepEqual(JSON.parse(saved).players[0].resources, JSON.parse(raw).players[0].resources);
  const restored = JSON.parse(engine.webPlanning(engine.stripSecret(saved, 0), 0));
  assert.deepEqual(restored.actions, actions);
  const resumed = JSON.parse(engine.tryMove(saved, settle, 1));
  assert.equal(resumed.action_plan, undefined);
  assert.deepEqual(resumed.players[0].resources, expected);
  assert.deepEqual(resumed.players[1].units, JSON.parse(resolved).players[1].units);
  assert.equal(resumed.players[0].cities.find((c: any) => c.position === 'D2').activations, 1);
  assert.equal(resumed.players[0].event_info.PublicEducation, 'used');
});
