import {test} from 'node:test';
import assert from 'node:assert/strict';
import {eventNotices,liveEventNotices} from './event-notices.ts';
import type {Game,View,LoggedAction,EventInfo} from './types.ts';
const famine={id:1,name:'Famine',rules:['Barbarians move.','Pay 1 food per city.']} as EventInfo;
const fire={id:2,name:'Fire',rules:['Lose 1 wood.']} as EventInfo;
const view={eventCatalog:[famine,fire]} as View;
function game(actions:LoggedAction[], log_index=actions.length):Game {
 return {players:[],log:[{age:1,rounds:[{round:1,turns:[{turn_type:{Player:0},actions}]}]}],log_index} as unknown as Game;
}
const trigger=(name:string,id:number):LoggedAction=>({log:[`A new game event has been triggered: ${name}`],items:[{player:0,Text:`triggers the event ${name}`},{player:0,origin:{Incident:id},Text:'resolve'}]});
test('live event reveals deduplicate log formats and do not repeat on acknowledgements',()=>{
 const before=game([]),after=game([trigger('Famine',1)]);
 assert.deepEqual(liveEventNotices(before,after,view).map(n=>n.rules),[famine.rules]);
 assert.deepEqual(liveEventNotices(after,after,view),[]);
 assert.deepEqual(liveEventNotices(after,before,view),[]);
 assert.deepEqual(liveEventNotices(null,after,view),[]);
 assert.deepEqual(liveEventNotices(null,after,{...view,pendingEvent:{id:1,player:0}}).map(n=>n.rules),[famine.rules]);
});
test('replay shows only events in the displayed interval and never undone triggers',()=>{
 const g=game([trigger('Famine',1),{},trigger('Fire',2)]);
 assert.deepEqual(eventNotices(g,view,0,1).map(n=>n.name),['Famine']);
 assert.deepEqual(eventNotices(g,view,1,2),[]);
 assert.deepEqual(eventNotices(g,view,1,3).map(n=>n.name),['Fire']);
 g.log_index=2;
 assert.deepEqual(eventNotices(g,view,1,3),[]);
});
test('same-name event variants use their own public origin and never a later event’s rules',()=>{
 const other={...famine,id:3,rules:['Gold deposits.','Pay 1 food per city.']};
 const catalog={eventCatalog:[famine,other]};
 assert.deepEqual(eventNotices(game([trigger('Famine',3)]),catalog,0,1)[0].rules,other.rules);
 const g=game([{log:['A new game event has been triggered: Famine']},trigger('Famine',3)]);
 assert.deepEqual(eventNotices(g,catalog,0,1)[0].rules,['Pay 1 food per city.']);
});
