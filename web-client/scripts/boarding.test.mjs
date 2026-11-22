import assert from 'node:assert/strict';
import { test } from 'node:test';
import { engine, fixture } from './boarding-fixture.mjs';
const move = (raw, action) => engine.tryMove(raw, JSON.stringify(action), 0);
const assignment = (pirates) => ({Movement:{Move:{units:[0,1,2,3],destination:'F6',boarding:[
  {units:[0,1],destination:'F6',[pirates?'embark_pirate':'embark_carrier_id']:pirates?2:10},
  {units:[2,3],destination:'F6',[pirates?'embark_pirate':'embark_carrier_id']:pirates?3:11},
]}}});
for (const pirates of [false,true]) {
  test(`compound boarding ${pirates?'pirates':'owned'}: invalid later load leaves all state and subsequent execution unchanged`, () => {
    const state = fixture(pirates,4), action = assignment(pirates), raw = JSON.stringify(state);
    const expected = move(raw, action);
    for (const change of [
      a => { a.boarding[1].units.push(0); },
      a => { a.boarding[1][pirates?'embark_pirate':'embark_carrier_id'] = 9999; },
      a => { a.boarding[0].units = [0]; a.boarding[1].units = [1,2,3]; },
      a => { a.boarding[1].destination = 'G7'; },
      a => { a.boarding[1].payment = {gold:1}; },
    ]) {
      const bad = structuredClone(action); change(bad.Movement.Move);
      assert.throws(() => move(raw,bad));
      assert.equal(JSON.stringify(state),raw);
      // Includes RNG state, claims, logs, action count, history and all private hands.
      assert.equal(move(raw,action),expected);
      for (const seat of [0,1,undefined]) assert.equal(engine.stripSecret(raw,seat),engine.stripSecret(JSON.stringify(state),seat));
    }
    const after = JSON.parse(expected);
    assert.equal(after.actions_left,2);
    assert.equal(after.state.Movement.movement_actions_left,2);
    assert.deepEqual(after.players[0].units.filter(u=>u.unit_type==='Ship').map(u=>u.carried_units.length),[2,2]);
    assert.equal(engine.logLength(expected),engine.logLength(raw)+1);
    const undone = move(expected,'Undo');
    assert.deepEqual(JSON.parse(undone).players,JSON.parse(engine.setPlayerSettings(raw,0,state.players[0].settings ?? {})).players);
    const redone = move(undone,'Redo');
    assert.deepEqual(JSON.parse(redone).players,after.players);
    assert.equal(JSON.parse(redone).actions_left,after.actions_left);
    const branched = move(undone,action);
    assert.deepEqual(JSON.parse(branched).players,after.players);
    assert.throws(()=>move(expected,action),'no same-space transfers');
  });
  test(`compound boarding ${pirates?'pirates':'owned'}: Tactics and per-passenger restrictions are not bypassed`, () => {
    const state = fixture(pirates,4);
    state.players[0].advances = state.players[0].advances.filter(a=>a!=='Tactics');
    assert.throws(()=>move(JSON.stringify(state),assignment(pirates)));
    const moved = fixture(pirates,4);
    moved.state = {Movement:{movement_actions_left:2,moved_units:[2]}};
    assert.throws(()=>move(JSON.stringify(moved),assignment(pirates)));
  });
}

test('Viking Longships retain three-passenger capacity in a compound assignment', () => {
  const state = fixture(false,4);
  state.players[0].civilization = 'Vikings';
  state.players[0].advances.push('WarShips');
  const action = assignment(false);
  action.Movement.Move.boarding[0].units = [0];
  action.Movement.Move.boarding[1].units = [1,2,3];
  const after = JSON.parse(move(JSON.stringify(state),action));
  assert.deepEqual(after.players[0].units.filter(u=>u.unit_type==='Ship').map(u=>u.carried_units.length),[1,3]);
});

for (const pirates of [false,true]) test(`compound boarding projects as one undoable queue entry (${pirates})`, async () => {
  const { ActionQueue } = await import('../src/action-queue.ts');
  const raw = JSON.stringify(fixture(pirates,4));
  const queue = new ActionQueue(engine,0,raw);
  const before = queue.projected;
  assert.deepEqual(queue.append(assignment(pirates)),{ok:true});
  assert.equal(queue.count,1);
  assert.equal(JSON.parse(queue.projected).actions_left,2);
  const result = queue.replay(raw);
  assert.equal(result.applied,1);
  assert.deepEqual(result.remaining,[]);
  assert.deepEqual(JSON.parse(result.raw).players,JSON.parse(move(raw,assignment(pirates))).players);
  queue.undo();
  assert.equal(queue.count,0);
  assert.equal(queue.projected,before);
});

test('pirate boarding claims the whole space, maps IDs separately, and enforces 4 per space / 8 total', () => {
  const state = fixture(true,4);
  const neutral = state.players.find(p=>p.civilization==='Pirates');
  // A colored carrier with a numerically colliding ID is never mistaken for a pirate.
  // Free passenger ID 2 before giving that same numeric ID to an owned ship.
  state.players[0].units[2].id = 4;
  state.players[0].units.push({id:2,position:'G7',unit_type:'Ship'});
  neutral.units.push({id:5,position:'F6',unit_type:'Ship'},{id:6,position:'F6',unit_type:'Ship'});
  const action = assignment(true);
  action.Movement.Move.units = [0,1,4,3];
  action.Movement.Move.boarding[1].units = [4,3];
  for(let id=30;id<33;id++)state.players[0].units.push({id,position:'G7',unit_type:'Ship'});
  state.players[0].next_unit_id=40;
  const after = JSON.parse(move(JSON.stringify(state),action));
  assert.equal(after.players[0].units.filter(u=>u.unit_type==='Ship').length,8);
  assert.equal(after.players.find(p=>p.civilization==='Pirates').units?.length??0,0);
  assert.equal(after.players[0].units.find(u=>u.id===2).position,'G7');
  assert.deepEqual(after.players[0].units.filter(u=>u.position==='F6').map(u=>u.id),[40,41,42,43]);
  const ninth = structuredClone(state);
  ninth.players[0].units.push({id:34,position:'G7',unit_type:'Ship'});
  assert.throws(()=>move(JSON.stringify(ninth),action));
  const fifth = structuredClone(state);
  fifth.players.find(p=>p.civilization==='Pirates').units.push({id:7,position:'F6',unit_type:'Ship'});
  fifth.players[0].units = fifth.players[0].units.filter(u=>u.id!==32);
  assert.throws(()=>move(JSON.stringify(fifth),action));
});

test('Navigator cannot be used to embark fresh land passengers', () => {
  const state = fixture(false,4);
  state.players[0].civilization = 'Carthage';
  state.players[0].event_info = {'Navigator active':'yes'};
  state.state = {Movement:{movement_actions_left:1,unit_only:true}};
  assert.throws(()=>move(JSON.stringify(state),assignment(false)));
});
