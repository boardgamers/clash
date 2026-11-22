// Explicit maintenance command only. Use the immutable published engine 0.4.165.
// CLASH_WASM=/absolute/path/to/0.4.165/package/server.js node scripts/generate-legacy-horse-master.mjs
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import {
  battle,
  respond,
  engine,
} from "../web-client/scripts/horse-master-fixture.mjs";
assert.equal(
  JSON.parse(
    readFileSync(join(dirname(process.env.CLASH_WASM), "package.json")),
  ).version,
  "0.4.165",
);
const pending = JSON.parse(battle());
const accepted = JSON.parse(respond(JSON.stringify(pending), { Bool: true }));
// A separate from-init legal history exercises reconstruction, without pretending
// that the synthetic Horse Master board can be reconstructed from its seed.
let history = await engine.init(2, [], {}, "legacy-finalization-165", {});
const cursors = [];
for (let i = 0; i < 2; i++) {
  history = engine.tryMove(
    history,
    JSON.stringify({ Playing: "EndTurn" }),
    engine.currentPlayer(history),
  );
  cursors.push({
    to: engine.logLength(history),
    state: JSON.parse(engine.replay(history, {})),
  });
}
// Synthetic board, real old-engine actions: not a from-init replay history.
writeFileSync(
  new URL(
    "../server/tests/fixtures/legacy_horse_master_165.json",
    import.meta.url,
  ),
  JSON.stringify(
    {
      engine: "0.4.165",
      pending,
      accepted,
      history: JSON.parse(history),
      cursors,
    },
    null,
    2,
  ) + "\n",
);
