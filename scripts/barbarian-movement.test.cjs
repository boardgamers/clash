// Synthetic public WASM captures for Barbarian movement fixes; no production saves.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');
const move = (g, a, player = 0) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(a), player));

const loadFixture = (rel) =>
  JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '..', 'server', 'tests', 'test_games', rel),
      'utf8',
    ),
  );

const barbUnitsAt = (g, pos) =>
  g.players
    .find((p) => p.civilization === 'Barbarians')
    .units.filter((u) => u.position === pos);

function setupMergingIncident(boundary) {
  const g = loadFixture('incidents/barbarians_move.json');
  if (boundary === undefined) delete g.barbarian_movement_from;
  else g.barbarian_movement_from = boundary;
  g.players[1].units = [];
  g.players[0].cities = [
    { position: 'A1', mood_state: 'Happy' },
    { position: 'C2', mood_state: 'Happy' },
  ];
  g.players[0].units = [];
  const barb = g.players.find((p) => p.civilization === 'Barbarians');
  barb.cities = [];
  barb.units = [
    { id: 10, position: 'A3', unit_type: 'Cavalry' },
    { id: 20, position: 'B2', unit_type: 'Infantry' },
  ];
  barb.next_unit_id = 21;
  return g;
}

test('merging Barbarian armies moves only unmoved units under new boundary and preserves legacy', () => {
  let g = setupMergingIncident(0);
  g = move(g, { Response: { SelectAdvance: 'Storage' } });
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  assert.deepEqual(
    barbUnitsAt(g, 'B2').map((u) => u.id),
    [10],
    'already-moved Cavalry stays at B2',
  );
  assert.deepEqual(
    barbUnitsAt(g, 'C2').map((u) => u.id),
    [20],
    'only unmoved Infantry advances to C2',
  );

  let legacy = setupMergingIncident(999999);
  legacy = move(legacy, { Response: { SelectAdvance: 'Storage' } });
  legacy = move(legacy, { Response: { SelectPositions: ['A3'] } });
  assert.deepEqual(barbUnitsAt(legacy, 'B2'), []);
  assert.deepEqual(
    barbUnitsAt(legacy, 'C2')
      .map((u) => u.id)
      .sort((a, b) => a - b),
    [10, 20],
  );
});

test('Barbarians move incident enforces stack limit 4 on destination choices', () => {
  const g0 = loadFixture('incidents/barbarians_move.json');
  g0.barbarian_movement_from = 0;
  g0.players[1].units = [];
  g0.players[1].cities = [];
  g0.players[0].cities = [
    { position: 'A1', mood_state: 'Happy' },
    { position: 'C1', mood_state: 'Happy' },
  ];
  g0.players[0].units = [];
  const barb = g0.players.find((p) => p.civilization === 'Barbarians');
  barb.cities = [];
  barb.units = [
    { id: 1, position: 'B2', unit_type: 'Cavalry' },
    { id: 2, position: 'B2', unit_type: 'Cavalry' },
    { id: 3, position: 'B1', unit_type: 'Infantry' },
    { id: 4, position: 'B1', unit_type: 'Infantry' },
    { id: 5, position: 'B1', unit_type: 'Infantry' },
    { id: 6, position: 'C2', unit_type: 'Infantry' },
    { id: 7, position: 'C2', unit_type: 'Infantry' },
  ];
  barb.next_unit_id = 8;

  let g = move(g0, { Response: { SelectAdvance: 'Storage' } });
  g = move(g, { Response: { SelectPositions: ['B2'] } });
  assert.equal(barbUnitsAt(g, 'B2').length, 0);
  assert.equal(barbUnitsAt(g, 'C2').length, 4);
  assert.equal(barbUnitsAt(g, 'B1').length, 3);
});

test('Mercenaries moving first army onto second does not move first army twice', () => {
  const setupMerc = (boundary) => {
    const g = loadFixture('action_cards/mercenaries.json');
    g.barbarian_movement_from = boundary;
    g.players[1].units = [];
    g.players[0].units = g.players[0].units.filter((u) => u.position !== 'B2');
    const barb = g.players.find((p) => p.civilization === 'Barbarians');
    barb.cities = [];
    barb.units = [
      { id: 10, position: 'A3', unit_type: 'Cavalry' },
      { id: 11, position: 'A3', unit_type: 'Cavalry' },
      { id: 20, position: 'B3', unit_type: 'Infantry' },
      { id: 21, position: 'B3', unit_type: 'Infantry' },
      { id: 30, position: 'B2', unit_type: 'Elephant' },
      { id: 31, position: 'B2', unit_type: 'Elephant' },
    ];
    barb.next_unit_id = 32;
    return g;
  };

  let g = move(setupMerc(0), { Playing: { ActionCard: 13 } });
  g = move(g, { Response: { SelectPositions: ['A3', 'B3'] } });
  g = move(g, { Response: { Payment: [{ ore: 2 }] } });
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  g = move(g, { Response: { SelectPositions: ['B3'] } });
  g = move(g, { Response: { SelectPositions: ['B2'] } });
  assert.equal((g.events || []).length, 0);
  assert.deepEqual(
    barbUnitsAt(g, 'B3')
      .map((u) => u.id)
      .sort((a, b) => a - b),
    [10, 11],
  );
  assert.deepEqual(
    barbUnitsAt(g, 'B2')
      .map((u) => u.id)
      .sort((a, b) => a - b),
    [20, 21, 30, 31],
  );

  let legacy = move(setupMerc(999999), { Playing: { ActionCard: 13 } });
  legacy = move(legacy, { Response: { SelectPositions: ['A3', 'B3'] } });
  legacy = move(legacy, { Response: { Payment: [{ ore: 2 }] } });
  legacy = move(legacy, { Response: { SelectPositions: ['A3'] } });
  legacy = move(legacy, { Response: { SelectPositions: ['B3'] } });
  assert.equal(barbUnitsAt(legacy, 'B3').length, 0);
  assert.equal(barbUnitsAt(legacy, 'A3').length, 4);
});

test('upgraded save stamps barbarian_movement_from and preserves it across undo/redo', () => {
  let g = setupMergingIncident(undefined);
  assert.equal(g.barbarian_movement_from, undefined);
  g = move(g, { Response: { SelectAdvance: 'Storage' } });
  const boundary = g.barbarian_movement_from;
  assert.equal(typeof boundary, 'number');
  g = move(g, { Response: { SelectPositions: ['A3'] } });
  assert.equal(g.barbarian_movement_from, boundary);
  assert.deepEqual(
    barbUnitsAt(g, 'B2').map((u) => u.id),
    [10],
  );
  const undone = move(g, 'Undo');
  assert.equal(undone.barbarian_movement_from, boundary);
  const redone = move(undone, 'Redo');
  assert.equal(redone.barbarian_movement_from, boundary);
  assert.deepEqual(
    barbUnitsAt(redone, 'B2').map((u) => u.id),
    [10],
  );
});
