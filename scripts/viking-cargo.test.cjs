// Synthetic public fixture; exercise the actual JSON/WASM move and save boundaries.
const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { resolve } = require("node:path");
const engine = require(process.env.CLASH_WASM || "../server/pkg/server.js");
const move = (g, action) =>
  JSON.parse(engine.move(JSON.stringify(g), JSON.stringify(action), 1));
function fixture(mixed) {
  const g = JSON.parse(
    readFileSync(
      resolve(
        __dirname,
        "../server/tests/test_games/civilizations/vikings/ship_construction.json",
      ),
    ),
  );
  [g.players[0], g.players[1]] = [g.players[1], g.players[0]];
  g.players.forEach((p, i) => (p.id = i));
  g.current_player_index = g.starting_player_index = 1;
  g.log[0].rounds[0].turns[0].turn_type.Player = 1;
  g.players[1].recruited_leaders = ["Ragnar"];
  g.players[1].units = [
    { id: 3, position: "D2", unit_type: "Ship" },
    { id: 4, position: "D2", unit_type: { Leader: "Ragnar" }, carrier_id: 3 },
  ];
  if (mixed)
    g.players[1].units.push({
      id: 5,
      position: "D2",
      unit_type: "Infantry",
      carrier_id: 3,
    });
  return g;
}
for (const mixed of [false, true])
  for (const settler of [false, true]) {
    test(`Ship Construction unloads ${mixed ? "mixed" : "leader-only"} cargo as ${settler ? "settler" : "infantry"} at seat 1 after save/resume`, () => {
      let g = fixture(mixed);
      const opponent = structuredClone(g.players[0]);
      g = move(g, {
        Movement: { Move: { units: [3], destination: "C2", payment: {} } },
      });
      assert.equal(engine.currentPlayer(JSON.stringify(g)), 1);
      assert.deepEqual(
        g.players[1].units[0].carried_units.map((u) => u.id),
        mixed ? [4, 5] : [4],
      );
      const pending = JSON.stringify(g);
      assert.match(pending, /Select units to convert to settlers/);
      // Restore the pending state, rather than reissuing movement or repairing cargo.
      g = move(JSON.parse(pending), {
        Response: { SelectUnits: settler ? [3] : [] },
      });
      const units = g.players[1].units;
      assert.deepEqual(
        units.map((u) => u.id),
        mixed ? [3, 4, 5] : [3, 4],
      );
      assert.equal(units[0].unit_type, settler ? "Settler" : "Infantry");
      assert.deepEqual(units[1].unit_type, { Leader: "Ragnar" });
      if (mixed) assert.equal(units[2].unit_type, "Infantry");
      assert.ok(
        units.every((u) => u.position === "C2" && u.carrier_id == null),
      );
      assert.deepEqual(g.players[0].units, opponent.units);
      assert.equal(g.state.Movement.movement_actions_left, 2);
      assert.deepEqual(g.state.Movement.moved_units, [3]);
      const log = JSON.stringify(g.log);
      const expected = `Unloaded ${mixed ? "1 infantry and " : ""}Ragnar Lodbrok from ships`;
      assert.equal(log.split(expected).length - 1, 1);
      assert.equal(
        log.split(
          `Converted 1 ship and to 1 ${settler ? "settler" : "infantry"}`,
        ).length - 1,
        1,
      );
      assert.ok(!(g.events || []).length, "conversion prompt resolved");
      assert.doesNotThrow(() => engine.webView(JSON.stringify(g), 1));
    });
  }
