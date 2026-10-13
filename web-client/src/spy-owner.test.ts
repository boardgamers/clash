import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
const engine=createRequire(import.meta.url)('../.engine/server.js');
test('Spies identifies each redacted-hand owner and uses that owner for objective progress',async()=>{
 const game=JSON.parse(readFileSync(new URL('../../server/tests/test_games/action_cards/spy.json',import.meta.url),'utf8'));
 game.players[0].objective_cards=[1];game.players[1].objective_cards=[2];
 game.players.push(...JSON.parse(await engine.init(2,[],{},'spy',{})).players.slice(2));
 let state=engine.tryMove(JSON.stringify(game),JSON.stringify({Playing:{ActionCard:7}}),0);
 state=engine.tryMove(state,JSON.stringify({Response:{Payment:[{culture_tokens:1}]}}),0);
 const view=JSON.parse(engine.webView(engine.stripSecret(state,0),0));
 for(const option of view.decision.options){
  const owner=option.value.ActionCard===1 || option.value.ObjectiveCard===1 ? 0 : 1;
  assert.equal(option.owner.player,owner);assert.equal(option.owner.you,owner===0);
  if(option.value.ObjectiveCard){
   const ownView=JSON.parse(engine.webView(engine.stripSecret(state,owner),owner));
   const card=ownView.objectiveCards.find((c:any)=>c.id===option.value.ObjectiveCard);
   assert.deepEqual(option.card.objectives.map((o:any)=>o.progress),card.objectives.map((o:any)=>o.progress));
   assert.deepEqual(option.card.objectives.map((o:any)=>o.conditionMet),card.objectives.map((o:any)=>o.conditionMet));
  }
 }
 assert.equal(JSON.parse(engine.webView(engine.stripSecret(state,1),1)).decision,null,'opponent must not receive Spies choices');
});
