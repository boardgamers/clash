import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const start = (count = 4, seed = 'private-civilization-draft') =>
  engine.init(count, [], { civilization: 'DraftThree' }, seed, {});
const view = (s: string, seat?: number) => JSON.parse(engine.webView(engine.stripSecret(s, seat), seat));
const choose = (s: string, seat: number, civilization: string) =>
  engine.tryMove(s, JSON.stringify({ ChooseCivilization: civilization }), seat);
const options = (s: string, seat: number): string[] => view(s, seat).civilizations.map((c: any) => c.name);
function unstarted(s: string) {
  const g = JSON.parse(s);
  assert.equal(g.state, 'ChooseCivilization');
  assert.equal(g.age, 0);
  assert.deepEqual(g.map.tiles, []);
  assert.equal(g.map.unexplored_blocks?.length ?? 0, 0);
  for (const p of g.players.slice(0, -2)) {
    assert.equal(p.civilization, 'Choose Civilization');
    assert.equal(p.cities?.length ?? 0, 0);
    assert.equal(p.units?.length ?? 0, 0);
    assert.equal(p.action_cards?.length ?? 0, 0);
    assert.equal(p.objective_cards?.length ?? 0, 0);
  }
}
for (const count of [2, 3, 4])
  test(`${count} players receive disjoint saved offers and can lock simultaneously before setup`, async () => {
    let s = await start(count);
    unstarted(s);
    const offers = Array.from({ length: count }, (_, i) => options(s, i));
    assert.ok(offers.every((o) => o.length === 3));
    assert.equal(new Set(offers.flat()).size, 3 * count);
    assert.deepEqual(
      engine.currentPlayer(s),
      Array.from({ length: count }, (_, i) => i),
    );
    for (const i of Array.from({ length: count }, (_, i) => count - i - 1)) {
      const own = view(s, i);
      assert.equal(own.civilizationDraft.waiting, false);
      assert.deepEqual(options(s, i), offers[i]);
      s = choose(s, i, offers[i][1]);
      if (i > 0) {
        unstarted(s);
        assert.deepEqual(
          engine.currentPlayer(s),
          Array.from({ length: i }, (_, p) => p),
        );
        assert.equal(view(s, i).civilizationDraft.chosen, offers[i][1]);
        assert.equal(view(s, i).civilizationDraft.waiting, true);
        assert.equal(view(s, i).canUndo, false);
        assert.deepEqual(options(s, i), offers[i]);
        assert.deepEqual(options(s, 0), offers[0]);
        // A reload round-trip retains the deal and the lock without drawing again.
        s = engine.setPlayerMetaData(s, 0, { name: 'Reloaded player' });
        assert.deepEqual(options(s, 0), offers[0]);
      }
    }
    const g = JSON.parse(s);
    assert.equal(g.state, 'Playing');
    assert.equal(g.age, 1);
    assert.equal(g.round, 1);
    assert.ok(g.map.tiles.length > 0);
    assert.equal(g.civilization_draft, undefined);
    assert.equal(engine.currentPlayer(s), g.starting_player_index);
    assert.ok(view(s, engine.currentPlayer(s)).canPlay);
    for (let i = 0; i < count; i++) {
      const p = g.players[i];
      assert.equal(p.civilization, offers[i][1]);
      assert.equal(p.cities.length, 1);
      assert.equal(p.units.length, 1);
      assert.equal(p.units[0].unit_type, 'Settler');
      assert.equal(p.action_cards.length, 1);
      assert.equal(p.objective_cards.length, 1);
      assert.equal(p.resources.food, 2);
      assert.ok(p.advances.includes('Farming') && p.advances.includes('Mining'));
      assert.equal(view(s, i).civilizationDraft, null);
    }
    assert.equal(view(s, engine.currentPlayer(s)).canUndo, false);
  });

test('private offers and locked choices never leak through player/spectator state, view, factions or logs', async () => {
  let s = await start(3);
  const offers = Array.from({ length: 3 }, (_, i) => options(s, i));
  s = choose(s, 2, offers[2][0]);
  assert.equal(JSON.parse(s).board_history, undefined, 'no empty-map or private draft frames');
  for (const seat of [undefined, 0, 1, 2]) {
    const stripped = engine.stripSecret(s, seat),
      g = JSON.parse(stripped);
    for (let p = 0; p < 3; p++)
      if (p !== seat) {
        assert.deepEqual(g.civilization_draft.offers[p], []);
        assert.equal(g.civilization_draft.choices[p], null);
        for (const civ of offers[p])
          assert.ok(!stripped.includes(`"${civ}"`), `${civ} leaked to seat ${seat}`);
      }
    assert.deepEqual(g.civilization_draft.ready, [false, false, true]);
    assert.equal(view(s, seat).civilizationDraft.chosen, seat === 2 ? offers[2][0] : null);
    assert.ok(engine.factions(s).every((c: string) => c === 'Choose Civilization'));
    assert.deepEqual(engine.logSlice(s, { start: 0, player: seat }), []);
  }
});

test('invalid and non-setup moves cannot bypass the private offer', async () => {
  let s = await start();
  const offered = options(s, 0)[0],
    foreign = options(s, 1)[0];
  for (const civ of [foreign, 'Barbarians', 'Pirates', 'Unknown'])
    assert.throws(() => choose(s, 0, civ), /three civilizations/);
  for (const seat of [4, 5, 99]) assert.throws(() => choose(s, seat, offered), /cannot choose/);
  for (const action of ['Undo', 'Redo', { Playing: 'EndTurn' }])
    assert.throws(() => engine.tryMove(s, JSON.stringify(action), 0), /three civilizations/);
  s = choose(s, 0, offered);
  const revised = options(s, 0)[1];
  const payload = JSON.stringify({ ChooseCivilization: revised });
  assert.equal(engine.canMoveOutOfTurn(s, payload, 0), true);
  assert.equal(engine.canMoveOutOfTurn(s, payload, 1), false);
  assert.equal(engine.canMoveOutOfTurn(s, JSON.stringify({ ChooseCivilization: foreign }), 0), false);
  assert.equal(engine.canMoveOutOfTurn(s, JSON.stringify('Undo'), 0), false);
  s = choose(s, 0, revised);
  assert.equal(engine.isLiveUpdate(s), true);
  assert.deepEqual(engine.currentPlayer(s), [1, 2, 3]);
  assert.equal(view(s, 0).civilizationDraft.chosen, revised);
  assert.equal(view(s, 1).civilizationDraft.chosen, null);
  for (const seat of [1, 2, 3]) s = choose(s, seat, options(s, seat)[0]);
  assert.equal(engine.isLiveUpdate(s), false);
  assert.equal(engine.canMoveOutOfTurn(s, payload, 0), false);
  assert.throws(() => choose(s, 0, revised));
});

test('submission order does not affect the map, decks, starting player or assignments', async () => {
  const initial = await start();
  const offers = Array.from({ length: 4 }, (_, i) => options(initial, i));
  const finish = (order: number[]) => order.reduce((s, i) => choose(s, i, offers[i][0]), initial);
  assert.deepEqual(JSON.parse(finish([0, 1, 2, 3])), JSON.parse(finish([3, 1, 0, 2])));
  const frames = JSON.parse(finish([0, 1, 2, 3])).board_history.frames;
  assert.equal(frames.length, 1);
  assert.equal(frames[0].actor, null);
  assert.equal(frames[0].title, 'Game setup');
  assert.ok(frames[0].tiles.length > 0);
});

test('a player leaving during the draft does not block the other choices', async () => {
  let s = await start(3);
  const offers = Array.from({ length: 3 }, (_, i) => options(s, i));
  s = await engine.dropPlayer(s, 1);
  assert.deepEqual(engine.currentPlayer(s), [0, 2]);
  unstarted(s);
  s = choose(s, 2, offers[2][0]);
  s = choose(s, 0, offers[0][0]);
  assert.equal(JSON.parse(s).state, 'Playing');
  assert.notEqual(engine.currentPlayer(s), 1);
  assert.ok(
    JSON.parse(s)
      .players.slice(0, 3)
      .every((p: any) => p.cities.length === 1),
  );
});
