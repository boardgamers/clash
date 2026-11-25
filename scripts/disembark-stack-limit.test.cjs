// Synthetic public WASM regression tests for disembarkation stacking-limit enforcement.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

const engine = require(process.env.CLASH_WASM || '../web-client/.engine/server.js');

const loadFixture = (rel) =>
  JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '..', 'server', 'tests', 'test_games', rel),
      'utf8',
    ),
  );

const queryMovement = (g, units, seat = 0) =>
  JSON.parse(
    engine.webQuery(
      JSON.stringify(g),
      seat,
      JSON.stringify({ kind: 'movement', units }),
    ),
  );

const tryMove = (g, action, seat = 0) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), seat));

const moveAction = (units, destination) => ({
  Movement: { Move: { units, destination, payment: {} } },
});

function setupDisembarkFixture(carriedTypes, friendlyOnLand = []) {
  const g = loadFixture('movement/movement.json');
  g.state = 'Playing';
  g.actions_left = 3;
  g.map.tiles = [
    ['D8', 'Water'],
    ['C8', 'Barren'],
  ];
  for (const p of g.players) {
    p.units = [];
    p.cities = [];
  }
  if (!g.players[0].advances.includes('Tactics')) {
    g.players[0].advances.push('Tactics');
  }
  if (carriedTypes.some((t) => typeof t === 'object' && t.Leader === 'Alexander')) {
    g.players[0].civilization = 'Greece';
    g.players[0].recruited_leaders = ['Alexander'];
  }

  const shipsNeeded = Math.max(1, Math.ceil(carriedTypes.length / 2));
  for (let s = 0; s < shipsNeeded; s++) {
    g.players[0].units.push({
      id: 100 + s,
      position: 'D8',
      unit_type: 'Ship',
    });
  }

  const carriedIds = [];
  carriedTypes.forEach((unit_type, idx) => {
    const id = idx;
    g.players[0].units.push({
      id,
      position: 'D8',
      unit_type,
      carrier_id: 100 + Math.floor(idx / 2),
    });
    carriedIds.push(id);
  });

  friendlyOnLand.forEach((unit_type, idx) => {
    g.players[0].units.push({
      id: 200 + idx,
      position: 'C8',
      unit_type,
    });
  });

  g.players[0].next_unit_id = 300;
  return { g, carriedIds };
}

test('disembarking >4 army units including Leader is excluded by webQuery and rejected by tryMove', () => {
  const { g, carriedIds } = setupDisembarkFixture([
    'Infantry',
    'Infantry',
    'Infantry',
    'Infantry',
    { Leader: 'Alexander' },
    'Settler',
  ]);

  const overStackQuery = queryMovement(g, carriedIds);
  assert.ok(
    !overStackQuery.destinations.some((d) => d.position === 'C8'),
    'webQuery must exclude C8 for 5 army units + Settler',
  );

  assert.throws(
    () => tryMove(g, moveAction(carriedIds, 'C8')),
    /stack limit exceeded|not found/i,
    'tryMove must reject disembarking 5 army units + Settler',
  );
});

test('disembarking 4 army units + Settler (with or without Leader) is allowed by webQuery and tryMove', () => {
  // 4 Infantry + 1 Settler
  {
    const { g } = setupDisembarkFixture([
      'Infantry',
      'Infantry',
      'Infantry',
      'Infantry',
      { Leader: 'Alexander' },
      'Settler',
    ]);
    const validIds = [0, 1, 2, 3, 5];
    const q = queryMovement(g, validIds);
    assert.ok(
      q.destinations.some((d) => d.position === 'C8'),
      'webQuery must include C8 for 4 Infantry + 1 Settler',
    );
    const after = tryMove(g, moveAction(validIds, 'C8'));
    for (const id of validIds) {
      const unit = after.players[0].units.find((u) => u.id === id);
      assert.equal(unit.position, 'C8');
      assert.equal(unit.carrier_id ?? null, null);
    }
  }

  // 3 Infantry + non-Xerxes Leader + 1 Settler
  {
    const { g } = setupDisembarkFixture([
      'Infantry',
      'Infantry',
      'Infantry',
      'Infantry',
      { Leader: 'Alexander' },
      'Settler',
    ]);
    const validIds = [0, 1, 2, 4, 5];
    const q = queryMovement(g, validIds);
    assert.ok(
      q.destinations.some((d) => d.position === 'C8'),
      'webQuery must include C8 for 3 Infantry + Leader + 1 Settler',
    );
    const after = tryMove(g, moveAction(validIds, 'C8'));
    for (const id of validIds) {
      const unit = after.players[0].units.find((u) => u.id === id);
      assert.equal(unit.position, 'C8');
      assert.equal(unit.carrier_id ?? null, null);
    }
  }
});

test('disembarking onto friendly army stack enforces limit for army units but allows Settlers', () => {
  // 2 Infantry onto 3 existing Infantry (5 > 4) rejected
  {
    const { g, carriedIds } = setupDisembarkFixture(
      ['Infantry', 'Infantry'],
      ['Infantry', 'Infantry', 'Infantry'],
    );
    const q = queryMovement(g, carriedIds);
    assert.ok(!q.destinations.some((d) => d.position === 'C8'));
    assert.throws(() => tryMove(g, moveAction(carriedIds, 'C8')));
  }

  // 1 Settler onto 4 existing Infantry allowed
  {
    const { g, carriedIds } = setupDisembarkFixture(
      ['Settler'],
      ['Infantry', 'Infantry', 'Infantry', 'Infantry'],
    );
    const q = queryMovement(g, carriedIds);
    assert.ok(q.destinations.some((d) => d.position === 'C8'));
    const after = tryMove(g, moveAction(carriedIds, 'C8'));
    const settler = after.players[0].units.find((u) => u.id === carriedIds[0]);
    assert.equal(settler.position, 'C8');
  }
});
