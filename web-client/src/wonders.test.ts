import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { officialWonderText, wonderName } from './wonder-names.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('official wonder names preserve saved IDs and appear in private and public views', async () => {
  const names = {
    Colosseum: 'Great Arena',
    Pyramids: 'Great Pyramid',
    GreatGardens: 'Great Gardens',
    GreatLibrary: 'Great Library',
    GreatLighthouse: 'Great Lighthouse',
    GreatMausoleum: 'Great Mausoleum',
    GreatStatue: 'Great Statue',
    GreatWall: 'Great Wall',
  };
  const game = JSON.parse(await engine.init(2, [], {}, 'wonder-names', {}));
  game.players[0].wonder_cards = Object.keys(names);
  const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  assert.deepEqual(Object.fromEntries(view.wonderCards.map((w: any) => [w.id, w.name])), names);
  for (const [id, name] of Object.entries(names)) assert.equal(wonderName(id), name);
  assert.equal(officialWonderText('Colosseum: built Pyramids'), 'Great Arena: built Great Pyramid');
});

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
