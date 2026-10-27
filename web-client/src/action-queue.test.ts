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
