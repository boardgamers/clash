import { test } from 'node:test';
import assert from 'node:assert/strict';
import { movementWaypoints } from './movement-animation.ts';
import { positionXY } from './model.ts';
import { seaEdgePoint } from './sea-routes.ts';
import type { Terrain } from './types.ts';
test('ships and carried units follow connected sea tiles around land in either direction', () => {
  const tiles: [string,Terrain][] = [['D2','Water'],['E2','Mountain'],['E3','Water'],['F2','Water']];
  const expected = ['D2','E3','F2'].map(positionXY);
  assert.deepEqual(movementWaypoints(tiles,'D2','F2',true),expected);
  assert.deepEqual(movementWaypoints(tiles,'D2','F2',false),expected);
  assert.deepEqual(movementWaypoints(tiles,'F2','D2',true),[...expected].reverse());
});
test('land movement visits intermediate land tiles and never cuts through sea', () => {
  const tiles: [string,Terrain][] = [['D2','Fertile'],['E2','Water'],['E3','Forest'],['F2','Mountain']];
  assert.deepEqual(movementWaypoints(tiles,'D2','F2',false),['D2','E3','F2'].map(positionXY));
});
test('Navigation travels along perimeter points rather than across intervening land', () => {
  const tiles: [string,Terrain][] = [['D2','Water'],['E2','Mountain'],['F2','Water']];
  const perimeter=['D2','E2','F2'];
  assert.deepEqual(movementWaypoints(tiles,'D2','F2',true),[]);
  assert.deepEqual(movementWaypoints(tiles,'D2','F2',true,[perimeter]),[positionXY('D2'),...perimeter.map(p=>seaEdgePoint(p,tiles)),positionXY('F2')]);
});
