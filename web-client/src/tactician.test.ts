import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'qin', {})).players.slice(2);
function battle() {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/tactics_cards/peltasts.json', import.meta.url),
      'utf8',
    ),
  );
  g.players.push(...structuredClone(npcs));
  g.players[0].civilization = 'China';
  for (let i = 0; i < 2; i++) {
    g.players[i].units = Array.from({ length: 4 }, (_, id) => ({
      id,
      unit_type: i === 0 && id === 3 ? { Leader: 'Qin' } : 'Infantry',
      position: i === 0 ? 'C2' : 'C1',
    }));
    g.players[i].advances = ['Tactics'];
    g.players[i].action_cards = [];
  }
  g.dice_roll_outcomes = Array(100).fill(4);
  const raw = JSON.stringify(g);
  const q = JSON.parse(engine.webQuery(raw, 0, JSON.stringify({ kind: 'movement', units: [0, 1, 2, 3] })));
  return engine.tryMove(
    raw,
    JSON.stringify(q.destinations.find((d: any) => d.position === 'C1' && d.attack).action),
    0,
  );
}
for (const declineFirst of [false, true])
  test(`Tactician can be paid once per battle (decline first=${declineFirst})`, () => {
    let raw = battle(),
      paid = 0,
      offers = 0,
      maxRound = 0;
    for (let n = 0; n < 20; n++) {
      const g = JSON.parse(raw),
        event = g.events?.at(-1),
        h = event?.handler;
      maxRound = Math.max(maxRound, event?.event_type?.CombatRoundEnd?.combat.stats.round ?? 0);
      if (!h) break;
      const r = h.request;
      let response;
      if (r.Payment) {
        assert.equal(h.origin.LeaderAbility, 'Tactician');
        assert.equal(paid, 0, 'Cannot offer Tactician again after paying');
        offers++;
        const pay = !(declineFirst && offers === 1);
        response = { Payment: [pay ? { culture_tokens: 2 } : {}] };
        if (pay) paid++;
      } else if (r.SelectUnits) {
        response = {
          SelectUnits: r.SelectUnits.choices
            .filter((id: number) => id !== 3)
            .slice(0, r.SelectUnits.needed.start),
        };
      } else if (r.BoolRequest) {
        response = { Bool: false };
      } else assert.fail(JSON.stringify(h));
      // Round trips through saved JSON must preserve the once-per-battle marker.
      raw = engine.tryMove(
        JSON.stringify(g),
        JSON.stringify({ Response: response }),
        engine.currentPlayer(raw),
      );
    }
    assert.equal(paid, 1);
    assert.equal(offers, declineFirst ? 2 : 1);
    assert(maxRound >= 2);
    assert(JSON.parse(battle()).events.at(-1).handler.request.Payment, 'A new battle offers Tactician again');
  });
