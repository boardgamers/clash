import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {writeFile} from 'node:fs/promises';
import {inspectEngine} from '@boardgamers/protocol/engine';
const engine=createRequire(import.meta.url)('../.engine/server.js');
const diagnostics=inspectEngine(engine);assert(diagnostics.compatible);
let raw=await engine.init(2,[],{},'release-check-2026-10-18',{});
const seat=engine.currentPlayer(raw);
assert.equal(typeof seat,'number');
for(let i=0;i<2;i++) raw=engine.move(raw,JSON.stringify({Playing:'EndTurn'}),engine.currentPlayer(raw));
assert.equal(engine.replay(raw,{}),engine.replay(raw,{}),'Replay must be deterministic');
const scenarios=[];
for(const player of [0,1,undefined]) {
 const view=engine.stripSecret(raw,player);
 assert.equal(typeof view,'string');
 if(engine.canLaunchAnalysisMode(raw)) {
   const options={seed:'release-simulation',...(player===undefined?{}:{player})};
   const a=engine.createAnalysisScenario(raw,options);
   assert.equal(a,engine.createAnalysisScenario(raw,options));
   const moved=engine.move(a,JSON.stringify({Playing:'EndTurn'}),engine.currentPlayer(a));
   assert.notEqual(moved,a);
   scenarios.push(player??'spectator');
 }
}
// Account setting revisions do not append game history or award another turn.
const before=engine.logLength(raw);
for(let n=0;n<100;n++)raw=engine.setPlayerSettings(raw,0,{skipRazeCity:n%2===0});
assert.equal(engine.logLength(raw),before);
let loop=await engine.init(2,[],{},'release-undo-check',{});
const loopSeat=engine.currentPlayer(loop), city=JSON.parse(loop).players[loopSeat].cities[0].position;
const collect=JSON.stringify({Playing:{Collect:{city_position:city,collections:[{position:city,pile:{food:1},times:1}],action_type:'Collect'}}});
let firstSize;
for(let n=0;n<100;n++) {
 loop=engine.move(loop,collect,loopSeat);
 loop=engine.move(loop,JSON.stringify('Undo'),loopSeat);
 if(n===0)firstSize=loop.length;
 assert.equal(loop.length,firstSize,'Collect/undo must not grow the save');
}
await writeFile(new URL('../release/engine-checks.json' ,import.meta.url),JSON.stringify({diagnostics,scenarios,deterministicReplay:true,settingRevisions:100,stableHistory:true,collectUndoCycles:100},null,2)+'\n');
console.log('Engine diagnostics, replay, seat/spectator analysis and bounded setting revisions passed');
