// Synthetic public WASM captures; no production saves.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const {engine, move, capture, city, setup} = require('./provinces-fixture.cjs');
for (const last of [false, true]) {
  test(`Leadership spends last culture; capture ${last ? 'last' : 'nonlast'} city is Neutral`, async () => {
    let g = await setup(1, 0, false, last);
    g.players[0].action_cards = [25];
    const actions = g.actions_left;
    g = move(g, {Playing: {ActionCard: 25}});
    g = move(g, {Response: {Payment: [{culture_tokens: 1}]}});
    assert.equal(g.players[0].resources?.culture_tokens || 0, 0);
    assert.equal(g.actions_left, actions + 1);
    g = capture(g);
    assert.equal(city(g).mood_state, 'Neutral');
    assert.equal((g.events || []).length, 0);
  });
}
for (const [culture, mood, arena] of [[0, 0, false], [0, 0, true], [1, 0, false], [0, 1, true]]) {
  for (const accept of culture || mood ? [false, true] : [false]) {
    test(`capture culture=${culture} mood=${mood} Arena=${arena} accept=${accept}`, async () => {
      let g = capture(await setup(culture, mood, arena));
      assert.equal(city(g).mood_state, 'Neutral');
      if (culture || mood) {
        assert.ok(g.events.length > 0);
        g = move(g, {Response: {Payment: [accept ? culture ? {culture_tokens: 1} : {mood_tokens: 1} : {}]}});
      }
      assert.equal(city(g).mood_state, accept ? 'Happy' : 'Neutral');
      assert.equal((g.players[0].resources?.culture_tokens || 0) + (g.players[0].resources?.mood_tokens || 0), culture + mood - Number(accept));
    });
  }
}

test('withdrawn Provinces owner deterministically declines optional Happy payment', async () => {
  const pending = capture(await setup(1, 0, false, false));
  const g = JSON.parse(await engine.dropPlayer(JSON.stringify(pending), 0));
  assert.equal(city(g).mood_state, 'Neutral');
  assert.equal(g.players[0].resources?.culture_tokens, 1);
  assert.deepEqual(JSON.parse(await engine.dropPlayer(JSON.stringify(pending), 0)), g);
});
