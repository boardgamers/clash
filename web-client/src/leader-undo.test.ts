import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import type { PlayerView } from './types';
const engine = createRequire(import.meta.url)('../.engine/server.js');

test('recruiting and undoing Pakal restores resources, recruitment and construction offers', async () => {
  let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'pakal-undo', {});
  for (const civilization of ['Maya', 'Carthage'])
    raw = engine.tryMove(
      raw,
      JSON.stringify({ ChooseCivilization: civilization }),
      engine.currentPlayer(raw),
    );
  const game = JSON.parse(raw);
  const seat = game.players.findIndex((p: { civilization: string }) => p.civilization === 'Maya');
  game.current_player_index = seat;
  game.players[seat].resources = { food: 4, ore: 4, wood: 4, mood_tokens: 3, culture_tokens: 3 };
  raw = JSON.stringify(game);
  const view = (state: string, player = seat) =>
    JSON.parse(engine.webView(engine.stripSecret(state, player), player));
  const before = view(raw);
  const roster = (state: string, player: number | undefined = seat) =>
    (JSON.parse(engine.webView(engine.stripSecret(state, player), player)).players as PlayerView[]).find(
      (p) => p.index === seat,
    )!.civilizationLeaders!;
  assert.deepEqual(
    roster(raw)
      .map((l) => l.id)
      .sort(),
    ['Pakal', 'SiyajKak', 'WakChanilAjaw'],
  );
  assert.ok(roster(raw).every((l) => !l.recruited && l.abilities.length === 2));
  const quote = JSON.parse(
    engine.webQuery(
      engine.stripSecret(raw, seat),
      seat,
      JSON.stringify({
        kind: 'recruit',
        city: before.cityActions[0].position,
        units: { leader: 'Pakal' },
        replaced: [],
      }),
    ),
  );
  const recruited = engine.tryMove(raw, JSON.stringify(quote.action), seat);
  assert(view(recruited).players[seat].leaders.some((leader: { id: string }) => leader.id === 'Pakal'));
  assert.equal(roster(recruited).length, 3, 'recruitment keeps the entire civilization roster visible');
  assert.deepEqual(
    roster(recruited)
      .filter((l) => l.recruited)
      .map((l) => l.id),
    ['Pakal'],
  );
  assert.deepEqual(roster(recruited, 1 - seat), roster(recruited), 'opponents see all public leader details');
  const spectator = JSON.parse(engine.webView(engine.stripSecret(recruited))).players[seat];
  assert.deepEqual(spectator.civilizationLeaders, roster(recruited));
  const logCursor = engine.logLength(recruited);
  const undone = engine.tryMove(recruited, JSON.stringify('Undo'), seat);
  // BGS requests the move response's journal delta after persisting the undo.
  assert(engine.logLength(undone) < logCursor);
  assert.deepEqual(engine.logSlice(undone, { start: logCursor, player: seat }), []);
  assert.deepEqual(JSON.parse(undone).players[seat].resources, game.players[seat].resources);
  assert.deepEqual(view(undone).cityActions, before.cityActions);
  assert.equal(view(undone).players[seat].leaders.length, 0);
  assert.equal(view(undone, 1 - seat).players[seat].leaders.length, 0);
  assert.deepEqual(roster(undone), roster(raw), 'undo restores the recruitment marker');
  const redone = engine.tryMove(undone, JSON.stringify('Redo'), seat);
  assert.deepEqual(view(redone).cityActions, view(recruited).cityActions);
  assert.deepEqual(roster(redone), roster(recruited));
  const retired = JSON.parse(redone);
  retired.players[seat].units = retired.players[seat].units.filter(
    (unit: { unit_type: unknown }) => typeof unit.unit_type === 'string',
  );
  assert.deepEqual(
    roster(JSON.stringify(retired)).map(({ reason, ...leader }) => leader),
    roster(recruited).map(({ reason, ...leader }) => leader),
    'a removed leader remains in the roster',
  );
  assert.equal(
    roster(JSON.stringify(retired)).find((leader) => leader.id === 'Pakal')!.reason,
    'Killed or replaced · Cannot recruit again',
  );
  assert.equal(view(JSON.stringify(retired)).players[seat].leaders.length, 0);
});

test('Pakal can build with culture replacing any pair of resources, only in his city', async () => {
  let raw = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'pakal-cost', {});
  for (const civilization of ['Maya', 'Carthage'])
    raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: civilization }), engine.currentPlayer(raw));
  const base = JSON.parse(raw), seat = base.players.findIndex((p: any) => p.civilization === 'Maya');
  base.current_player_index = seat;
  const player = base.players[seat], city = player.cities[0];
  const other = structuredClone(city); other.position = 'C2';
  player.cities.push(other);
  player.units = [{ id: 0, position: city.position, unit_type: { Leader: 'Pakal' } }];
  player.recruited_leaders = ['Pakal'];
  player.advances.push('Tactics');
  city.mood_state = 'Happy';
  for (const resource of ['food', 'wood', 'ore', 'gold']) {
    const game = structuredClone(base);
    game.players[seat].resources = { [resource]: 1, culture_tokens: 1 };
    const state = JSON.stringify(game);
    const cities = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)).cityActions;
    const fortress = cities.find((c: any) => c.position === city.position).buildings.find((b: any) => b.name === 'Fortress');
    assert.equal(fortress.reason, null, resource);
    assert.deepEqual(fortress.payment, { [resource]: 1, culture_tokens: 1 });
    const after = JSON.parse(engine.tryMove(state, JSON.stringify(fortress.choices[0].action), seat));
    assert.equal(after.players[seat].cities[0].city_pieces.fortress, seat);
    assert.equal(after.players[seat].resources?.culture_tokens ?? 0, 0);
    const elsewhere = cities.find((c: any) => c.position === other.position).buildings.find((b: any) => b.name === 'Fortress');
    assert.equal(elsewhere.reason, 'Not enough resources');
  }
});
