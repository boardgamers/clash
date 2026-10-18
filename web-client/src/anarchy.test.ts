import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'anarchy-regression', {})).players.slice(2);
function afterAnarchy(count = 1) {
  const game = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/incidents/trojan/anarchy.outcome.json', import.meta.url),
      'utf8',
    ),
  );
  for (const npc of npcs)
    if (!game.players.some((p: any) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  game.players[0].incident_tokens = 1;
  game.players[0].special_victory_points[0].points = count;
  game.permanent_effects = [{ Anarchy: { player: 0, advances_lost: count } }];
  game.actions_left = 3;
  return game;
}
const view = (g: any): View => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const move = (g: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(g), JSON.stringify(action), 0));
const research = (g: any, id: string) => {
  const advance = view(g).advances.find((a) => a.id === id)!;
  assert.ok(advance.action, `${id} is available`);
  return move(g, advance.action);
};

test('Anarchy suppresses the government research warning and leaves the event marker untouched', () => {
  const game = afterAnarchy();
  game.players[0].advances.push('Philosophy', 'Draft');
  const options = view(game).advances;
  // Anarchy removed Voting: its cube may also fund a different government.
  for (const id of ['Voting', 'Dogma', 'Nationalism']) {
    const advance = options.find((a) => a.id === id)!;
    assert.ok(advance.action, id);
    assert.equal(advance.triggersEvent, false, id);
  }
  assert.equal(options.find((a) => a.id === 'Engineering')!.triggersEvent, true);
  const after = research(game, 'Dogma');
  assert.equal(after.players[0].incident_tokens, 1);
  assert.deepEqual(after.permanent_effects ?? [], []);
  assert.deepEqual(after.incidents_left, game.incidents_left);
  assert.equal(after.rng, game.rng);
  const entries = after.log.flatMap((a: any) =>
    a.rounds.flatMap((r: any) => r.turns.flatMap((t: any) => t.actions ?? [])),
  );
  const gained = entries.at(-1).items.find((i: any) => i.Advance?.advance === 'Dogma');
  assert.equal(
    gained.Advance.incident_token,
    'NoChange',
    'The journal must not claim an event marker was spent',
  );
});

test('the warning returns only after the last Anarchy cube has been used', () => {
  let game = afterAnarchy(2);
  game = research(game, 'Dogma');
  assert.equal(game.permanent_effects[0].Anarchy.advances_lost, 1);
  assert.equal(view(game).advances.find((a) => a.id === 'Devotion')!.triggersEvent, false);
  game = research(game, 'Devotion');
  assert.equal(game.players[0].incident_tokens, 1);
  assert.deepEqual(game.permanent_effects ?? [], []);
  assert.equal(view(game).advances.find((a) => a.id === 'Fanaticism')!.triggersEvent, true);
});

test('another player researching government neither uses nor removes your Anarchy cubes', () => {
  const game = afterAnarchy();
  game.players[1].special_victory_points = [{ points: 2, origin: { Incident: 44 }, attribution: 'Events' }];
  game.players[0].special_victory_points = [];
  game.permanent_effects = [{ Anarchy: { player: 1, advances_lost: 2 } }];
  assert.equal(view(game).advances.find((a) => a.id === 'Dogma')!.triggersEvent, true);
  game.players[0].incident_tokens = 2;
  const after = research(game, 'Dogma');
  assert.equal(after.players[0].incident_tokens, 1);
  assert.deepEqual(after.permanent_effects, game.permanent_effects);
});

test('Anarchy uses only the researching player’s cube when multiple players are affected', () => {
  const game = afterAnarchy();
  game.players[1].special_victory_points = [{ points: 2, origin: { Incident: 44 }, attribution: 'Events' }];
  const other = { Anarchy: { player: 1, advances_lost: 2 } };
  game.permanent_effects.unshift(other);
  const after = research(game, 'Dogma');
  assert.equal(after.players[0].incident_tokens, 1);
  assert.deepEqual(after.permanent_effects, [other]);
});

test('a card-granted government advance uses an Anarchy cube without adding an event marker', () => {
  let game = afterAnarchy();
  game.players[0].action_cards = [2];
  game = move(game, { Playing: { ActionCard: 2 } });
  if (view(game).decision?.fields.length)
    game = move(game, { Response: { Payment: [{ culture_tokens: 1 }] } });
  assert.equal(view(game).advances.find((a) => a.id === 'Dogma')!.triggersEvent, false);
  game = research(game, 'Dogma');
  assert.equal(game.players[0].incident_tokens, 1);
  assert.deepEqual(game.permanent_effects ?? [], []);
});

test('a saved Anarchy point restores its missing cube for previews and actual research', () => {
  const game = afterAnarchy();
  delete game.permanent_effects;
  assert.equal(view(game).advances.find((a) => a.id === 'Dogma')!.triggersEvent, false);
  const after = research(game, 'Dogma');
  assert.equal(after.players[0].incident_tokens, 1);
  assert.deepEqual(after.incidents_left, game.incidents_left);
  assert.equal(after.rng, game.rng);
  assert.deepEqual(after.permanent_effects ?? [], []);
  assert.deepEqual(after.players[0].special_victory_points ?? [], []);
  assert.equal(view(after).advances.find((a) => a.id === 'Devotion')!.triggersEvent, true);

  const undone = move(after, 'Undo');
  assert.equal(undone.players[0].special_victory_points[0].points, 1);
  assert.equal(view(undone).advances.find((a) => a.id === 'Dogma')!.triggersEvent, false);
  const redone = move(undone, 'Redo');
  assert.equal(redone.players[0].incident_tokens, 1);
  assert.deepEqual(redone.permanent_effects ?? [], []);
  assert.deepEqual(redone.players[0].special_victory_points ?? [], []);
});

test('restoring missing Anarchy cubes is per player and does not duplicate surviving cubes on reload', () => {
  const game = afterAnarchy(3);
  game.permanent_effects[0].Anarchy.advances_lost = 1;
  game.players[1].special_victory_points = [{ points: 2, origin: { Incident: 44 }, attribution: 'Events' }];
  game.permanent_effects.push({ Anarchy: { player: 1, advances_lost: 2 } }, 'TrojanHorse');
  const loaded = JSON.parse(engine.stripSecret(JSON.stringify(game), 0));
  const cubes = (g: any, seat: number) =>
    g.permanent_effects.reduce(
      (sum: number, e: any) => sum + (e.Anarchy?.player === seat ? e.Anarchy.advances_lost : 0),
      0,
    );
  assert.equal(cubes(loaded, 0), 3);
  assert.equal(cubes(loaded, 1), 2);
  assert.ok(loaded.permanent_effects.includes('TrojanHorse'));
  const reloaded = JSON.parse(engine.stripSecret(JSON.stringify(loaded), 0));
  assert.deepEqual(reloaded.permanent_effects, loaded.permanent_effects);
  assert.deepEqual(reloaded.players[0].special_victory_points, game.players[0].special_victory_points);
});

test('unrelated event points do not restore Anarchy cubes', () => {
  const game = afterAnarchy();
  delete game.permanent_effects;
  game.players[0].special_victory_points[0].origin = { Incident: 42 };
  assert.equal(view(game).advances.find((a) => a.id === 'Dogma')!.triggersEvent, true);
});
