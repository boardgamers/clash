import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
const engine=createRequire(import.meta.url)('../.engine/server.js');
const npcs=JSON.parse(await engine.init(2,[],{},'revolution',{})).players.slice(2);
function move(state:string, response:unknown) { return engine.tryMove(state,JSON.stringify({Response:response}),0); }
function request(state:string) { return JSON.parse(state).events?.at(-1)?.handler?.request; }
for(const first of [false,true]) for(const second of [false,true]) {
  test(`Revolution sacrifices independently prevent action loss=${first}, government change=${second}`,()=>{
    const game=JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/civil_war/revolution.json',import.meta.url),'utf8'));
    game.players.push(...npcs);
    let state=engine.tryMove(JSON.stringify(game),JSON.stringify({Playing:{Advance:{advance:'Storage',payment:{gold:2}}}}),0);
    const actions=JSON.parse(state).actions_left;
    const one=request(state).SelectUnits.choices[0];
    state=move(state,{SelectUnits:first?[one]:[]});
    const two=request(state).SelectUnits.choices[0];
    state=move(state,{SelectUnits:second?[two]:[]});
    assert.equal(JSON.parse(state).actions_left,actions-(first?0:1));
    assert.equal(request(state)==='ChangeGovernment',!second);
  });
}
test('saved step two with an erased first-sacrifice flag preserves its earlier sacrifice',()=>{
  const game=JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/civil_war/revolution.json',import.meta.url),'utf8'));
  game.players.push(...npcs);
  let state=engine.tryMove(JSON.stringify(game),JSON.stringify({Playing:{Advance:{advance:'Storage',payment:{gold:2}}}}),0);
  const actions=JSON.parse(state).actions_left;
  state=move(state,{SelectUnits:[request(state).SelectUnits.choices[0]]});
  const legacy=JSON.parse(state);delete legacy.events.at(-1).event_type.Incident.sacrifice;
  state=move(JSON.stringify(legacy),{SelectUnits:[]});
  assert.equal(JSON.parse(state).actions_left,actions);
  assert.equal(request(state),'ChangeGovernment');
});
