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
test('explicit passenger choice preserves the other passenger (1+1 and 2+0)', () => {
  const initial = boardingPlan([1,2], 'E6', query([2,2]))!;
  const split = boardingPlan([1,2], 'E6', query([2,2]), {unit:2,carrier:1}, initial)!;
  assert.deepEqual(split.find(g => g.carrier === 0)?.units, [1]);
  assert.deepEqual(split.find(g => g.carrier === 1)?.units, [2]);
  const packed = boardingPlan([1,2], 'E6', query([2,2]), {unit:2,carrier:0}, split)!;
  assert.equal(packed.length, 1);
  assert.equal(packed[0].units.length, 2);
});
test('neutral and owned carrier IDs occupy separate namespaces', () => {
  const pirateQuery = (ids: number[]) => query([2,2])(ids).map((d,i) => i ? {...d,carrier:null,pirateCarrier:0} : d);
  const plan = boardingPlan([1,2,3,4], 'E6', pirateQuery)!;
  assert.deepEqual(plan.map(g => [g.carrier,g.units.length]), [[0,2],[-1,2]]);
});
