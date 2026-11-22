// Public synthetic board; all combat actions use the real engine.
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
export const engine = createRequire(import.meta.url)(process.env.CLASH_WASM ?? '../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'horse-master', {})).players.slice(2);
export const items = (g) =>
  g.log.flatMap((a) =>
    a.rounds.flatMap((r) => r.turns.flatMap((t) => (t.actions ?? []).flatMap((a) => a.items ?? []))),
  );
export function respond(raw, response) {
  return engine.tryMove(raw, JSON.stringify({ Response: response }), engine.currentPlayer(raw));
}
export function battle(leader = 'Attila', cavalry = true) {
  const g = JSON.parse(
    readFileSync(new URL('../../server/tests/test_games/combat/retreat.json', import.meta.url), 'utf8'),
  );
  g.players.push(...structuredClone(npcs));
  for (let p = 0; p < 2; p++) {
    g.players[p].civilization = p === 0 ? 'India' : 'Huns';
    g.players[p].advances = ['Farming', 'Mining', 'Tactics', ...(p === 1 ? ['SteelWeapons'] : [])];
    g.players[p].units = [
      { id: 0, position: p === 0 ? 'C2' : 'C1', unit_type: p === 0 ? 'Infantry' : { Leader: leader } },
      { id: 1, position: p === 0 ? 'C2' : 'C1', unit_type: p === 0 || !cavalry ? 'Infantry' : 'Cavalry' },
    ];
    g.players[p].next_unit_id = 2;
    g.players[p].action_cards = [p === 0 ? 31 : 9];
  }
  g.dice_roll_outcomes = [10, 2, 9, 3, 9];
  let raw = engine.tryMove(
    JSON.stringify(g),
    JSON.stringify({ Movement: { Move: { units: [0, 1], destination: 'C1', payment: {} } } }),
    0,
  );
  raw = respond(raw, { SelectHandCards: [{ ActionCard: 31 }] });
  raw = respond(raw, { Payment: [{ ore: 1 }] });
  return respond(raw, { SelectHandCards: [{ ActionCard: 9 }] });
}
