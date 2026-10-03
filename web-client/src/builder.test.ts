import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const serialize = (game: unknown) => JSON.stringify(game);
const view = (game: unknown, seat: number | undefined = 0) =>
  JSON.parse(engine.webView(engine.stripSecret(serialize(game), seat), seat));
const move = (game: unknown, action: unknown, seat = 0) =>
  JSON.parse(engine.tryMove(serialize(game), serialize(action), seat));

async function fixture(name: string) {
  const game = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  const npcs = JSON.parse(await engine.init(2, [], {}, 'builder-fixtures', {})).players.slice(2);
  game.players.push(...npcs);
  return game;
}

test('Builder keeps peaceful halves of objective cards in the private hand', async () => {
  const game = JSON.parse(await engine.init(2, [], { variant: 'Builder' }, 'builder-objectives', {}));
  game.players[0].objective_cards = [10, 20, 24, 38];
  assert.deepEqual(
    view(game).objectiveCards.map((card: any) => card.objectives.map((o: any) => o.name)),
    [
      ['Optimized Storage'],
      ['Science Focus'],
      ['Government', 'Barbarian Conquest'],
      ['High Culture', 'Sea Cleansing'],
    ],
  );
  game.players[1].objective_cards = [];
  assert.deepEqual(view(game, 1).objectiveCards, []);
  const publicGame = engine.stripSecret(serialize(game), undefined);
  assert.deepEqual(JSON.parse(engine.webView(publicGame, undefined)).objectiveCards, []);
  assert.equal(JSON.parse(publicGame).options.variant, 'Builder');
  game.options.variant = 'Standard';
  assert.equal(view(game).objectiveCards[0].objectives[1].name, 'Naval Assault');
});

test('Builder does not offer hostile map destinations and rejects a stale attack action', async () => {
  const game = await fixture('combat/remove_casualties_attacker');
  const query = () =>
    JSON.parse(
      engine.webQuery(
        engine.stripSecret(serialize(game), 0),
        0,
        serialize({ kind: 'movement', units: [0, 1, 2, 3] }),
      ),
    );
  const attack = query().destinations.find((d: any) => d.position === 'C1');
  assert.ok(attack?.attack);
  game.options = { variant: 'Builder' };
  assert.ok(!query().destinations.some((d: any) => d.position === 'C1'));
  assert.throws(() => move(game, attack.action), /Builder/);
});

test('Builder allows Persian conversion but excludes targets that would start a player battle', async () => {
  const game = await fixture('base/cultural_influence_instant');
  game.current_player_index = 0;
  const player = game.players[0];
  player.civilization = 'Persia';
  player.advances = [...new Set([...player.advances, 'Myths', 'Priesthood'])];
  player.resources.culture_tokens = 7;
  player.units = [];
  game.players[1].units = [
    { id: 0, position: 'B3', unit_type: 'Infantry' },
    { id: 1, position: 'B3', unit_type: 'Infantry' },
  ];
  game.players[1].next_unit_id = 2;
  game.dice_roll_outcomes = [10];
  const conversion = (state: unknown) =>
    view(state).influence.find((i: any) => i.variant === 'Zoroastrianism');
  const attack = conversion(game);
  assert.ok(attack);
  assert.doesNotThrow(() => move(game, attack.action), 'Standard permits the conversion battle');
  game.options = { variant: 'Builder' };
  assert.equal(conversion(game), undefined);
  assert.throws(() => move(game, attack.action), /Builder.*battle/);

  const barbarians = structuredClone(game);
  barbarians.players[2].units = barbarians.players[1].units;
  barbarians.players[2].next_unit_id = 2;
  barbarians.players[1].units = [];
  assert.ok(conversion(barbarians), 'conversion may still provoke a barbarian battle');
  assert.doesNotThrow(() => move(barbarians, conversion(barbarians).action));

  game.players[1].units.pop();
  const peaceful = conversion(game);
  assert.ok(peaceful, 'converting a lone unit does not start a battle');
  const after = move(game, peaceful.action);
  assert.ok(after.players[0].units.some((u: any) => u.position === 'B3' && u.unit_type === 'Infantry'));
  assert.equal((after.players[1].units ?? []).length, 0);
});
