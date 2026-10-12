import { test } from 'node:test';
import assert from 'node:assert/strict';
import { boardingPlan } from './boarding.ts';
import type { MoveDestination } from './types.ts';
const query = (capacity: number[], forbidden: number[] = []) => (ids: number[]): MoveDestination[] =>
  ids.some(id => forbidden.includes(id)) ? [] : capacity.flatMap((slots, carrier) => ids.length <= slots ? [{position:'E6',carrier,terrain:'Water',payment:{},attack:false,action:{}} as MoveDestination] : []);
test('boarding respects remaining capacity including occupied ships', () => {
  const plan = boardingPlan([1,2,3], 'E6', query([1,2]))!;
  assert.equal(plan.find(g=>g.carrier===0)!.units.length,1);
  assert.equal(plan.find(g=>g.carrier===1)!.units.length,2);
  assert.equal(boardingPlan([1,2,3], 'E6', query([1,1])),null);
  assert.equal(boardingPlan([1,2,3], 'E6', query([0,2])),null);
});
test('boarding can redistribute a passenger but cannot bypass a unit restriction', () => {
  const plan = boardingPlan([1,2,3], 'E6', query([2,2]), {unit:1,carrier:1})!;
  assert(plan.find(g=>g.carrier===1)!.units.includes(1));
  assert.equal(boardingPlan([1,2,3], 'E6', query([2,2],[2])),null);
  assert.equal(boardingPlan([1,2,3], 'E6', query([2,2]),{unit:1,carrier:9}),null);
});
