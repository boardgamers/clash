// Real production WASM, isolated states only: never submit live-game moves.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
const engine = createRequire(import.meta.url)(process.env.CLASH_ENGINE ?? '../.engine/server.js');
const template = JSON.parse(
  await readFile(new URL('../../server/tests/test_games/movement/movement.json', import.meta.url)),
);
const query = (state, units = [0]) =>
  JSON.parse(engine.webQuery(JSON.stringify(state), 0, JSON.stringify({ kind: 'movement', units })))
    .destinations;
const move = (state, action, seat = 0) =>
  JSON.parse(engine.move(JSON.stringify(state), JSON.stringify(action), seat));
const action = (destination, units = [0], payment = {}) => ({
  Movement: { Move: { units, destination, payment } },
});
function fixture(terrain, exhausted, unitType = 'Infantry') {
  const s = structuredClone(template);
  s.state = 'Playing';
  s.actions_left = 3;
  s.map.tiles = [
    ['A1', 'Barren'],
    ['B1', exhausted ? { Exhausted: terrain } : terrain],
    ['C2', 'Barren'],
    ['B2', 'Barren'],
  ];
  for (const p of s.players) {
    p.cities = [];
    p.units = [];
  }
  s.players[0].advances.push('Tactics');
  s.players[0].units = [
    { id: 0, position: 'A1', unit_type: unitType },
    { id: 1, position: 'B1', unit_type: 'Infantry' },
  ];
  s.players[1].units = [{ id: 0, position: 'B2', unit_type: 'Infantry' }];
  return s;
}
for (const exhausted of [false, true]) {
  for (const terrain of ['Forest', 'Mountain', 'Fertile']) {
    for (const type of ['Infantry', 'Settler', { Leader: 'Caesar' }]) {
      let s = fixture(terrain, exhausted, type);
      assert(query(s, [1]).some((d) => d.position === 'C2'));
      const entry = query(s).find((d) => d.position === 'B1');
      assert(entry);
      s = move(s, entry.action);
      assert(!query(s).some((d) => d.position === 'C2'), 'same-action unit identity');
      s = move(s, { Movement: 'Stop' });
      if (terrain === 'Mountain')
        assert.throws(() => engine.tryMove(JSON.stringify(s), JSON.stringify(action('C2')), 0));
      else assert.doesNotThrow(() => engine.tryMove(JSON.stringify(s), JSON.stringify(action('C2')), 0));
      if (type !== 'Settler' && terrain !== 'Fertile')
        assert.throws(() => engine.tryMove(JSON.stringify(s), JSON.stringify(action('B2')), 0));
      assert.equal(
        query(s).some((d) => d.position === 'C2'),
        terrain !== 'Mountain',
      );
      if (type !== 'Settler')
        assert.equal(
          query(s).some((d) => d.position === 'B2'),
          terrain === 'Fertile',
        );
      if (terrain === 'Mountain') assert(entry.terrainNotes.some((n) => n.includes('cannot move again')));
      if (terrain === 'Forest') assert(entry.terrainNotes.length);
      assert(
        query(s, [1]).some((d) => d.position === 'C2'),
        'different units unaffected',
      );
      assert.equal(
        query(s, [0, 1]).some((d) => d.position === 'C2'),
        terrain !== 'Mountain',
      );
      if (terrain === 'Fertile') {
        s.players[1].cities = [
          { position: 'B2', mood_state: 'Neutral', city_pieces: { wonders: ['GreatGardens'] } },
        ];
        assert(!query(s).some((d) => d.position === 'B2'), 'exhausted Plains still protects Great Gardens');
      }
      s = move(s, { Playing: 'EndTurn' });
      assert.deepEqual(s.players[0].units[0].movement_restrictions ?? [], []);
    }
    if (terrain === 'Fertile') continue;
    let s = fixture(terrain, exhausted);
    s.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
    s.players[0].advances.push('Roads');
    const offers = query(s).filter((d) => d.position === 'B1');
    assert(offers.some((d) => !Object.keys(d.payment).length));
    const paid = offers.find((d) => d.payment.food === 1 && d.payment.ore === 1);
    assert(paid, 'paid adjacent Roads terrain bypass remains available');
    s = move(s, paid.action);
    s = move(s, { Movement: 'Stop' });
    assert(query(s).some((d) => d.position === 'C2'));
    assert(query(s).some((d) => d.position === 'B2'));
    for (const restriction of ['Battle', 'Navigator']) {
      const stopped = structuredClone(s);
      stopped.players[0].units[0].movement_restrictions = [restriction];
      assert.equal(query(stopped).length, 0, 'Roads cannot bypass battle/navigation');
    }
  }
  for (const type of ['Settler', { Leader: 'Pakal' }, 'Infantry']) {
    let s = fixture('Mountain', exhausted, type);
    s.players[0].civilization = 'Maya';
    s.players[0].advances.push('Irrigation');
    s.players[0].special_advances = ['Terracing'];
    s = move(s, action('B1'));
    s = move(s, { Movement: 'Stop' });
    assert.equal(
      query(s).some((d) => d.position === 'C2'),
      type !== 'Infantry',
    );
  }
  let nomad = fixture('Mountain', exhausted);
  nomad.players[0].civilization = 'Huns';
  nomad.players[0].advances.push('Storage');
  nomad.players[0].cities = [{ position: 'A1', mood_state: 'Happy' }];
  nomad.players[0].resources.ore = 0;
  nomad = move(nomad, { Movement: { Move: { city: 'A1', units: [], destination: 'B1', payment: {} } } });
  assert.equal(nomad.players[0].cities[0].nomad_mountain, true);
  assert.equal(nomad.players[0].resources.ore ?? 0, exhausted ? 0 : 1);
  nomad = move(nomad, { Movement: 'Stop' });
  const cityRoutes = JSON.parse(
    engine.webQuery(JSON.stringify(nomad), 0, JSON.stringify({ kind: 'movement', city: 'B1', units: [] })),
  );
  assert.equal(cityRoutes.destinations.length, 0);
  let s = fixture('Mountain', exhausted, { Leader: 'Hannibal' });
  s.players[0].civilization = 'Carthage';
  s.players[0].units.push({ id: 2, position: 'A1', unit_type: 'Elephant' });
  s = move(s, action('B1', [0, 2]));
  s = move(s, { Movement: 'Stop' });
  assert(query(s, [0, 2]).some((d) => d.position === 'C2'));
}
console.log(
  'Exhausted/ordinary terrain: actual WASM moves, turn stops, identity, leaders/settlers, Great Gardens, Roads and exceptions passed',
);
