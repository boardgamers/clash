import { test } from 'node:test';
import assert from 'node:assert/strict';
import { militarySummary, strategyTiles, strategyDescription } from './strategy.ts';
import type { Player, Game } from './types.ts';
const player: Player = {
  id: 0,
  civilization: 'Maya',
  units: [
    { id: 0, position: 'B2', unit_type: 'Infantry' },
    { id: 1, position: 'B2', unit_type: { Leader: 'Pakal' } },
    { id: 2, position: 'B2', unit_type: 'Settler' },
    {
      id: 3,
      position: 'C2',
      unit_type: 'Ship',
      carried_units: [
        { id: 4, unit_type: 'Elephant' },
        { id: 5, unit_type: 'Cavalry' },
        { id: 6, unit_type: 'Settler' },
      ],
    },
  ],
};
test('army totals include leaders and passengers exactly once and exclude ships and settlers', () => {
  assert.deepEqual(militarySummary(player), { army: 4, aboard: 2, ships: 1, settlers: 2 });
  assert.deepEqual(militarySummary(player, 'B2'), { army: 2, aboard: 0, ships: 0, settlers: 1 });
  assert.deepEqual(
    militarySummary({
      ...player,
      units: [...player.units!, { id: 4, position: 'C2', unit_type: 'Elephant', carrier_id: 3 }],
    }),
    militarySummary(player),
  );
});
test('strategy markers use public positions, retain different city and army owners, and omit unexplored tiles', () => {
  const opponent: Player = {
    id: 1,
    civilization: 'Carthage',
    cities: [
      {
        position: 'B2',
        mood_state: 'Happy',
        city_pieces: { fortress: 1, academy: 0, wonders: ['Pyramids'] },
      },
    ],
  };
  const map: Game['map'] = {
    tiles: [
      ['B2', 'Forest'],
      ['C2', 'Water'],
      ['D2', 'Unexplored'],
      ['E2', { Exhausted: 'Mountain' }],
    ],
  };
  const tiles = strategyTiles({ map, players: [player, opponent] });
  assert.equal(tiles.length, 3);
  assert.equal(tiles[0].occupants.length, 2);
  assert.equal(tiles[0].occupants[1].size, 4);
  assert.match(strategyDescription(tiles[0]), /Carthage · city size 4, Fortress/);
  assert.match(strategyDescription(tiles[1]), /2 army \(2 aboard\) · 1 ships · 1 settlers/);
  assert.equal(strategyDescription(tiles[2]), 'Exhausted mountain');
  assert.doesNotMatch(strategyDescription(tiles[0]), /B2|Pakal/);
});
