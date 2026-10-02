import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tileUnitStacks } from './tile-units.ts';
import type { Player } from './types.ts';

test('tile summaries group units by owner and type, including leaders without internal identifiers', () => {
  const players: Player[] = [
    {
      id: 0,
      civilization: 'Maya',
      units: [
        { id: 0, position: 'B2', unit_type: 'Infantry' },
        { id: 1, position: 'B2', unit_type: 'Infantry' },
        { id: 2, position: 'B2', unit_type: { Leader: 'KInichJanaabPakal' } },
        { id: 3, position: 'C2', unit_type: 'Elephant' },
      ],
    },
    { id: 1, civilization: 'Barbarians', units: [{ id: 0, position: 'B2', unit_type: 'Infantry' }] },
  ];
  const stacks = tileUnitStacks(players, 'B2');
  assert.deepEqual(
    stacks.map((s) => [s.player.civilization, s.groups.map((g) => [g.name, g.count])]),
    [
      [
        'Maya',
        [
          ['Infantry', 2],
          ['Leader', 1],
        ],
      ],
      ['Barbarians', [['Infantry', 1]]],
    ],
  );
  assert.deepEqual(tileUnitStacks(players, 'D2'), []);
});

test('passengers stay separate from surface troops and duplicate public representations count once', () => {
  const player: Player = {
    id: 0,
    civilization: 'Rome',
    units: [
      {
        id: 0,
        position: 'B2',
        unit_type: 'Ship',
        carried_units: [
          { id: 1, unit_type: 'Infantry' },
          { id: 2, unit_type: 'Settler' },
        ],
      },
      { id: 1, position: 'B2', unit_type: 'Infantry', carrier_id: 0 },
      { id: 3, position: 'B2', unit_type: 'Infantry' },
    ],
  };
  for (const units of [player.units, [...player.units!].reverse()]) {
    const groups = tileUnitStacks([{ ...player, units }], 'B2')[0].groups;
    assert.equal(
      groups.reduce((n, g) => n + g.count, 0),
      4,
    );
    assert.deepEqual(
      groups
        .filter((g) => g.aboard)
        .map((g) => g.name)
        .sort(),
      ['Infantry', 'Settler'],
    );
    assert.equal(groups.find((g) => g.name === 'Infantry' && !g.aboard)?.count, 1);
  }
});

test('pirate ships retain their identity without marking passengers as pirates', () => {
  const players: Player[] = [
    {
      id: 0,
      civilization: 'Vikings',
      units: [
        {
          id: 0,
          position: 'B2',
          unit_type: 'Ship',
          pirate: true,
          carried_units: [{ id: 1, unit_type: 'Infantry' }],
        },
        { id: 2, position: 'B2', unit_type: 'Ship' },
      ],
    },
    { id: 3, civilization: 'Pirates', units: [{ id: 0, position: 'B2', unit_type: 'Ship' }] },
  ];
  const stacks = tileUnitStacks(players, 'B2');
  assert.deepEqual(
    stacks[0].groups.map((g) => [g.name, g.pirate]),
    [
      ['Pirate ship', true],
      ['Infantry', false],
      ['Ship', false],
    ],
  );
  assert.equal(stacks[1].groups[0].pirate, true);
});
