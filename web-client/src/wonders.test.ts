import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('built wonder effects are public for both players and spectators; private cards stay private', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'public-wonders', {}));
  game.players[0].cities[0].city_pieces = { wonders: ['GreatLighthouse', 'Pyramids'] };
  game.players[0].wonder_cards = ['GreatLibrary'];
  game.players[1].wonder_cards = ['GreatStatue'];
  for (const seat of [0, 1, undefined]) {
    const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat));
    assert.deepEqual(
      view.builtWonders.map((w: any) => w.id),
      ['GreatLighthouse', 'Pyramids'],
    );
    assert.match(view.builtWonders[0].description, /Activate the city/);
    assert.ok(Math.abs(view.builtWonders[1].builtPoints - 5.1) < 0.00001);
    assert.equal(view.builtWonders[1].ownedPoints, 0);
    assert.deepEqual(
      view.wonderCards.map((w: any) => w.id),
      seat === undefined ? [] : seat === 0 ? ['GreatLibrary'] : ['GreatStatue'],
    );
  }
});
