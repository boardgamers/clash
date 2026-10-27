import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'pirate-great-person', {})).players.slice(2);
for (const card of [55, 56, 58]) for (const poor of [false, true]) test(`claiming Great Person ${card} during status does not skip the other player's pirate raid (no resources ${poor})`, () => {
  const g = JSON.parse(readFileSync(new URL('../../server/tests/test_games/incidents/pirates_spawn.json', import.meta.url), 'utf8'));
  g.players = [...g.players.slice(0, 2), ...structuredClone(npcs)];
  g.players[0].civilization = 'Phoenicia';
  g.players[1].civilization = 'Vikings';
  g.players[1].resources = poor ? {} : { food: 2 };
  g.players[1].cities[0].mood_state = 'Happy';
  g.incidents_left = [card];
  let raw = JSON.stringify(g);
  const apply = (action: unknown) => { raw = engine.tryMove(raw, JSON.stringify(action), engine.currentPlayer(raw)); };
  const request = () => JSON.parse(raw).events?.at(-1)?.handler?.request;
  apply({Response:{SelectAdvance:'Storage'}});
  apply({Response:{SelectPositions:['A2']}});
  apply({Response:{SelectPositions:['D1']}});
  assert.equal(engine.currentPlayer(raw), 0);
  assert.ok(request().Payment[0].name.includes('pirates'));
  apply({Response:{Payment:[{food:1}]}});
  apply({Response:{Payment:[{culture_tokens:1}]}});
  assert.equal(engine.currentPlayer(raw), 1, 'the next affected player must resolve the raid');
  if (poor) {
    if (request()?.SelectPositions) {
      assert.ok(request().SelectPositions.choices.includes('C1'));
      apply({Response:{SelectPositions:['C1']}});
    }
    assert.equal(JSON.parse(raw).players[1].cities[0].mood_state, 'Neutral');
  } else {
    assert.ok(request().Payment[0].name.includes('pirates'));
    apply({Response:{Payment:[{food:1}]}});
    assert.equal(JSON.parse(raw).players[1].resources.food, 1);
  }
  assert.ok(JSON.parse(raw).players[0].action_cards.includes(card + 100));
  assert.ok(!JSON.stringify(request()).includes('Pay to gain the Action Card'));
});
