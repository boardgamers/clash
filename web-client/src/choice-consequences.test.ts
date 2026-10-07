import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'choice-consequences', {})).players.slice(2);
const move = (game: any, action: unknown) =>
  JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify(action), 0));
const view = (game: any, seat = 0) =>
  JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), seat), seat));
function fixture(name: string) {
  const game = JSON.parse(
    readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  game.players = [...game.players.slice(0, 2), ...structuredClone(npcs)];
  return game;
}
function guillotine() {
  const game = fixture('incidents/trojan/guillotine');
  // This older fixture predates tracking previously recruited leaders.
  game.players[0].recruited_leaders = ['Caesar'];
  return move(game, { Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } });
}

test('Guillotine explains the permanent leader loss before either response', () => {
  const decision = view(guillotine()).choiceDecision;
  assert.equal(decision.binary, true);
  assert.deepEqual(
    decision.choices.map((c: any) => c.name),
    ['Choose a new leader', 'Gain 2 victory points'],
  );
  assert.match(decision.choices[0].description, /All other unused leaders become unavailable/);
  assert.match(decision.choices[1].description, /cannot recruit leaders for the rest of the game/);
  assert.deepEqual(
    decision.choices.map((c: any) => c.action),
    [{ Response: { Bool: true } }, { Response: { Bool: false } }],
  );
});

for (const takeLeader of [false, true]) {
  test(`Guillotine's ${takeLeader ? 'new leader' : 'victory points'} choice retains every leader with the correct reason`, () => {
    let game = guillotine();
    const choice = view(game).choiceDecision.choices[takeLeader ? 0 : 1];
    game = move(game, choice.action);
    if (takeLeader) game = move(game, { Response: { SelectUnitType: { Leader: 'Augustus' } } });
    assert.deepEqual(game.players[0].available_leaders ?? [], []);
    const reasons = Object.fromEntries(view(game).cityActions[0].leaders.map((l: any) => [l.id, l.reason]));
    assert.deepEqual(reasons, {
      Augustus: takeLeader ? 'Already on board at A1' : 'Unavailable after Guillotine',
      Caesar: 'Killed or replaced · Cannot recruit again',
      Sulla: 'Unavailable after Guillotine',
    });
    // Permanent availability is also visible when inspecting another civilization.
    const publicLeaders = view(game, 1).players[0].civilizationLeaders;
    assert.deepEqual(Object.fromEntries(publicLeaders.map((l: any) => [l.id, l.reason])), reasons);
    const selected = game.players[0].units.find((u: any) => u.unit_type?.Leader === 'Augustus');
    assert.equal(!!selected, takeLeader);
  });
}

test('recruitment distinguishes a leader on the board, a previously used leader and an unused leader', () => {
  const game = fixture('incidents/trojan/guillotine');
  game.players[0].recruited_leaders = ['Caesar', 'Sulla'];
  game.players[0].available_leaders = ['Augustus'];
  game.players[0].cities[0].mood_state = 'Happy';
  const leaders = view(game).cityActions[0].leaders;
  assert.equal(leaders.length, 3);
  assert.equal(leaders.find((l: any) => l.id === 'Caesar').reason, 'Already on board at A1');
  assert.equal(
    leaders.find((l: any) => l.id === 'Sulla').reason,
    'Killed or replaced · Cannot recruit again',
  );
  assert.equal(leaders.find((l: any) => l.id === 'Augustus').reason, null);
  game.players[0].resources = {};
  const poor = view(game).cityActions[0].leaders;
  assert.equal(poor.find((l: any) => l.id === 'Augustus').reason, 'Not enough resources');
  assert.equal(poor.find((l: any) => l.id === 'Caesar').reason, 'Already on board at A1');
});

test('Great Athlete describes both conversion directions instead of treating No as skipping', () => {
  let game = fixture('incidents/great_persons/great_athlete');
  game = move(game, { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } });
  game = move(game, { Response: { Payment: [{ culture_tokens: 1 }] } });
  game = move(game, { Playing: { ActionCard: 156 } });
  const decision = view(game).choiceDecision;
  assert.equal(decision.binary, true);
  assert.deepEqual(
    decision.choices.map((c: any) => c.name),
    ['Culture → mood tokens', 'Mood → culture tokens'],
  );
  for (const choice of decision.choices) {
    assert.match(choice.description, /Choose how many/);
    const after = move(game, choice.action);
    assert.ok(view(after).decision.fields.length, 'Either option leads to choosing an exchange amount');
  }
});

test('Great Engineer explains the cost of constructing and the consequence of skipping', () => {
  let game = fixture('incidents/great_persons/great_engineer');
  for (const action of [
    { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } },
    { Response: { Payment: [{ culture_tokens: 1 }] } },
    { Playing: { ActionCard: 126 } },
    { Response: { SelectAdvance: 'Engineering' } },
  ])
    game = move(game, action);
  const decision = view(game).choiceDecision;
  assert.equal(decision.choices[0].name, 'Construct a building');
  assert.match(decision.choices[0].description, /normal resource cost/);
  assert.equal(decision.choices[1].name, 'Skip construction');
  assert.match(decision.choices[1].description, /Keep your resources/);
  const before = game.players[0].resources;
  const declined = move(game, decision.choices[1].action);
  assert.deepEqual(declined.players[0].resources, before);
  assert.ok(!declined.permanent_effects?.some((e: any) => e.Construct === 'GreatEngineer'));
  const accepted = move(game, decision.choices[0].action);
  assert.ok(accepted.permanent_effects.some((e: any) => e.Construct === 'GreatEngineer'));
});
