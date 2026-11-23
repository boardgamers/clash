// Actual production JSON/WASM boundaries. No wall-clock expiration is simulated:
// only the platform's explicit dropPlayer notification authorizes automation.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const engine = require(process.env.CLASH_WASM || '../server/pkg/server.js');
function fixture(name) {
  const g = JSON.parse(readFileSync(resolve(__dirname, '../server/tests/test_games/', name + '.json')));
  const third = structuredClone(g.players[0]);
  Object.assign(third, {id:2,civilization:'Persia',units:[],cities:[{position:'B2',mood_state:'Happy'}],action_cards:[],objective_cards:[]});
  g.players.splice(2,0,third);
  g.players.forEach((p,i)=>p.id=i);
  return g;
}
const move = (g,a,p) => JSON.parse(engine.move(JSON.stringify(g), JSON.stringify(a), p));
const drop = async (g,p) => JSON.parse(await engine.dropPlayer(JSON.stringify(g),p));
const active = g => engine.currentPlayer(JSON.stringify(g));
const clocks = g => engine.timeIncrements(JSON.stringify(g));
function battle() {
  const g = fixture('tactics_cards/peltasts');
  for (const p of [0,1]) g.players[p].advances.push('SteelWeapons');
  g.players[0].action_cards=[3,6,11];g.players[1].action_cards=[4,7,23];
  return move(g,{Movement:{Move:{units:[0,1,2],destination:'C1',payment:{}}}},0);
}
function defender(g) {
  return move(move(g,{Response:{Payment:[{}]}},0),{Response:{SelectHandCards:[]}},0);
}
test('drop notification drains optional defender prompts, preserves clocks, saves ordinary actions',async()=>{
  let g=defender(battle());
  const clock=clocks(g), ore=g.players[1].resources.ore, cards=g.players[1].action_cards;
  const start=performance.now();
  g=await drop(g,1);
  assert.ok(performance.now()-start<5000,'well within the host 10-second deadline');
  assert.notEqual(active(g),1);
  assert.equal(g.players[1].resources.ore,ore);
  assert.deepEqual(g.players[1].action_cards,cards);
  assert.deepEqual(clocks(g),clock);
  assert.deepEqual(await drop(g,1),g);
});
test('future dropped defender drains at ordinary move boundary, never answering living attacker',async()=>{
  let g=battle();const before=g.events;
  g=await drop(g,1);assert.deepEqual(g.events,before);assert.equal(active(g),0);
  g=defender(g);assert.notEqual(active(g),1);assert.equal(clocks(g)[1],0);
});
test('mandatory casualties resolve, living retreat remains open',async()=>{
  let g=fixture('combat/retreat');
  g.players[0].units=g.players[0].units.filter(u=>u.id<4).map(u=>({...u,unit_type:'Infantry'}));
  g.players[1].units=Array.from({length:4},(_,id)=>({id,position:'C1',unit_type:id===3?'Infantry':'Cavalry'}));
  g.dice_roll_outcomes=Array(8).fill(4);
  g=move(g,{Movement:{Move:{units:[0,1,2,3],destination:'C1',payment:{}}}},0);
  assert.equal(active(g),1);
  g=await drop(g,1);
  assert.equal(g.players[1].units.length,2);assert.equal(g.players[0].units.length,2);
  assert.equal(active(g),0);
  assert.ok(g.events.at(-1).handler.request.BoolRequest);
});
test('withdrawn attacker does not steal living defender choice or change suspended owner',async()=>{
  let g=defender(battle());const events=g.events;
  g=await drop(g,0);assert.equal(g.current_player_index,0);assert.equal(active(g),1);assert.deepEqual(g.events,events);
  g=move(g,{Response:{Payment:[{}]}},1);
  g=move(g,{Response:{SelectHandCards:[]}},1);
  assert.notEqual(active(g),0);
});
test('zero surviving seats produce terminal tied rankings',async()=>{
  let g=await drop(battle(),0);g=await drop(g,1);
  assert.equal(g.state,'Finished');assert.equal(engine.rankings(JSON.stringify(g))[2],1);
  g=await drop(g,2);assert.deepEqual(engine.rankings(JSON.stringify(g)),[1,1,1]);
});
test('pending status advance is completed for its saved actor, not the successor',async()=>{
  let g=move(fixture('status_phase/free_advance'),{Playing:'EndTurn'},0);
  assert.equal(active(g),1);const advances=g.players[0].advances;
  g=await drop(g,1);assert.deepEqual(g.players[0].advances,advances);assert.notEqual(active(g),1);
});
test('captured city still gives dropped loser a mandatory settler',async()=>{
  let g=fixture('combat/capture_and_raze_city');
  g.players[1].cities.push({position:'B1',mood_state:'Happy'});
  g=move(g,{Movement:{Move:{units:[0,1,2,3],destination:'C1',payment:{}}}},0);
  assert.equal(active(g),1);
  const settlers=(g.players[1].units||[]).filter(u=>u.unit_type==='Settler').length;
  g=await drop(g,1);
  assert.equal((g.players[1].units||[]).filter(u=>u.unit_type==='Settler').length,settlers+1);
  assert.notEqual(active(g),1);
});
test('government already paid for completes without a second token payment',async()=>{
  let g=move(fixture('status_phase/change_government'),{Response:{SelectPositions:[]}},1);
  if(active(g)===2) g=move(g,{Response:{SelectPositions:[]}},2);
  g=move(g,{Response:{Payment:[{culture_tokens:1,mood_tokens:1}]}},0);
  const resources=g.players[0].resources;
  g=await drop(g,0);
  assert.equal(g.players[0].resources.mood_tokens,resources.mood_tokens);
  assert.equal(g.players[0].resources.culture_tokens,resources.culture_tokens);
  assert.notEqual(active(g),0);
});
test('multiple inactive seats skip ordinary turns across round boundaries',async()=>{
  let g=JSON.parse(await engine.init(4,[],{},'withdrawal-four-seats',{}));
  // init uses the default simultaneous civilization draft; use its offers.
  if(g.civilization_draft) {
    for(let p=0;p<4;p++) g=move(g,{ChooseCivilization:g.civilization_draft.offers[p][0]},p);
  }
  const first=g.current_player_index;
  const dead=[(first+1)%4,(first+2)%4];
  g=await drop(g,dead[0]);g=await drop(g,dead[1]);
  for(let i=0;i<4;i++) {
    assert.ok(!dead.includes(active(g)));
    assert.ok(!(g.events||[]).length,'ordinary starting turns for this fixed seed');
    g=move(g,{Playing:'EndTurn'},active(g));
  }
  assert.equal(g.round,3);
  assert.ok(!dead.includes(active(g)));
  for(const p of dead) assert.equal(clocks(g)[p],0);
});
test('draft withdrawal locks only departed offers and ends at one survivor',async()=>{
  let g=JSON.parse(await engine.init(4,[],{civilization:'DraftThree'},'withdrawal-draft',{}));
  assert.ok(g.civilization_draft);
  const offers=structuredClone(g.civilization_draft.offers);
  g=await drop(g,0);
  assert.equal(g.civilization_draft.ready[0],true);
  assert.equal(g.civilization_draft.choices[0],offers[0][0]);
  assert.deepEqual(active(g),[1,2,3]);
  g=await drop(g,1);assert.deepEqual(active(g),[2,3]);
  g=await drop(g,2);assert.equal(g.state,'Finished');
  assert.equal(engine.rankings(JSON.stringify(g))[3],1);
});

for (const legacy of [false, true]) test(`withdrawn Horse Master defender declines pending modifier (${legacy ? 'legacy' : 'finalized'})`, async () => {
  let g=fixture('combat/retreat');
  g.players[0].civilization='India';g.players[1].civilization='Huns';
  for(const p of [0,1]) {
    g.players[p].advances=['Farming','Mining','Tactics'];
    g.players[p].units=[{id:0,position:p===0?'C2':'C1',unit_type:p===0?'Infantry':{Leader:'Attila'}},{id:1,position:p===0?'C2':'C1',unit_type:p===0?'Infantry':'Cavalry'}];
    g.players[p].next_unit_id=2;
  }
  g.players[1].advances.push('SteelWeapons');
  g.players[0].action_cards=[31];g.players[1].action_cards=[9];
  g.dice_roll_outcomes=[10,2,9,3,9];
  if(legacy)g.finalized_combat_from=10000;
  g=move(g,{Movement:{Move:{units:[0,1],destination:'C1',payment:{}}}},0);
  for(const r of [{SelectHandCards:[{ActionCard:31}]},{Payment:[{ore:1}]},{SelectHandCards:[{ActionCard:9}]}]) g=move(g,{Response:r},active(g));
  assert.match(g.events.at(-1).handler.request.BoolRequest,/Horse Master/);
  const expected=move(structuredClone(g),{Response:{Bool:false}},1);
  const before=clocks(g);g=await drop(g,1);
  assert.deepEqual(clocks(g),before);
  assert.deepEqual(g.players[0].units,expected.players[0].units);
  assert.deepEqual(g.players[1].units,expected.players[1].units);
  assert.notEqual(active(g),1);
});

test('private early QueueMoves update waits, then withdrawal resolves saved actor and resumes only living owner plan', async () => {
  const q=require('./postcombat-queue-fixture.cjs');
  let g=q.battle('win',false,g=>{
    const third=structuredClone(g.players[0]);
    Object.assign(third,{id:2,civilization:'Persia',units:[],cities:[{position:'B2',mood_state:'Happy'}],action_cards:[],objective_cards:[]});
    g.players.splice(2,0,third);g.players.forEach((p,i)=>p.id=i);
  });
  const info=q.planning(g);assert.ok(info);
  const events=structuredClone(g.events), before=clocks(g);
  g=move(g,{QueueMoves:{key:info.key,actions:[{Movement:'Stop'}],waiting:true}},0);
  assert.deepEqual(g.events,events);assert.deepEqual(clocks(g),before);
  assert.equal(q.stripped(g,1).action_plan,undefined);
  g=await drop(g,1);
  assert.equal(g.current_player_index,0);assert.equal(active(g),0);
  assert.equal(g.state,'Playing');assert.equal(g.action_plan,undefined);
  assert.deepEqual(clocks(g),before);
});

test('JSON full and partial replay retain the exact drop boundary and seal undo', async () => {
  let g=JSON.parse(await engine.init(4,[],{},'withdrawal-json-replay',{}));
  g=move(g,{Playing:'EndTurn'},active(g));
  const cursor=engine.logLength(JSON.stringify(g));
  const seat=(active(g)+1)%4;
  g=await drop(g,seat);
  const boundary=structuredClone(g.withdrawals);
  assert.throws(()=>move(g,'Undo',active(g)),'withdrawal seals earlier history');
  for(let n=0;n<2;n++)g=move(g,{Playing:'EndTurn'},active(g));
  const raw=JSON.stringify(g);
  const full=JSON.parse(engine.replay(raw,{}));
  assert.deepEqual(full.withdrawals,boundary);assert.deepEqual(full.dropped_players,[seat]);
  assert.equal(full.current_player_index,g.current_player_index);assert.equal(full.round,g.round);
  const before=JSON.parse(engine.replay(raw,{to:cursor-1}));
  assert.ok(!(before.dropped_players||[]).includes(seat));assert.equal(before.withdrawals,undefined);
  const at=JSON.parse(engine.replay(raw,{to:cursor}));
  assert.deepEqual(at.withdrawals,boundary);assert.deepEqual(at.dropped_players,[seat]);
  assert.deepEqual(clocks(at),clocks(g));
});
