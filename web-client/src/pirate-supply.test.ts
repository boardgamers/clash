import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const engine=createRequire(import.meta.url)('../.engine/server.js');
const npcs=JSON.parse(await engine.init(2,[],{},'pirate-supply',{})).players.slice(2);
for (const held of [0,1,2,3,4]) for (const neutral of new Set([0,4-held])) test(`Pirate event respects ${held} allied ships with ${neutral} neutral ships`,()=>{
 const g=JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/pirates_spawn.json',import.meta.url),'utf8'));
 for(const npc of npcs) if(!g.players.some((p:any)=>p.civilization===npc.civilization)) g.players.push({...npc,id:g.players.length});
 const p=g.players[0], pirates=g.players.find((p:any)=>p.civilization==='Pirates');
 p.civilization='Carthage';p.advances.push('Fishing','Navigation');
 p.units.push({id:30,unit_type:'Settler',position:'D1'});
 for(let i=0;i<held;i++)p.units.push({id:40+i,unit_type:'Ship',position:'D1',pirate:true});
 p.next_unit_id=50;
 pirates.units=Array.from({length:neutral},(_,i)=>({id:i,unit_type:'Ship',position:'D2'}));
 pirates.held_units={ships:held};pirates.next_unit_id=20;
 let state=engine.tryMove(JSON.stringify(g),JSON.stringify({Response:{SelectAdvance:'Storage'}}),0);
 for(let i=0;i<8;i++) {
  const current=JSON.parse(state), npc=current.players.find((p:any)=>p.civilization==='Pirates');
  assert.ok((npc.units??[]).length+(npc.held_units?.ships??0)<=4,'never exceed the physical supply');
  const req=current.events?.at(-1)?.handler?.request;
  if(req?.SelectUnits) {
   const amount=Math.min(2,neutral);
   state=engine.tryMove(state,JSON.stringify({Response:{SelectUnits:req.SelectUnits.choices.slice(0,amount)}}),0);
  } else if(req?.SelectPositions && current.events.at(-1).handler.request.SelectPositions.description?.includes('Pirate')) {
   state=engine.tryMove(state,JSON.stringify({Response:{SelectPositions:[req.SelectPositions.choices[0]]}}),0);
  } else break;
 }
 const final=JSON.parse(state), npc=final.players.find((p:any)=>p.civilization==='Pirates');
 assert.equal((npc.units??[]).filter((u:any)=>u.id>=20).length,Math.min(2,4-held));
 assert.equal(final.players[0].units.filter((u:any)=>u.pirate).length,held,'allies are untouched');
});
