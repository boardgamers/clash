import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { journal } from './journal.ts';
import type { EventInfo, Game, LoggedAction, View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (name: string) =>
  readFileSync(new URL(`../../server/tests/test_games/incidents/${name}.json`, import.meta.url), 'utf8');
const advance = { Playing: { Advance: { advance: 'Storage', payment: { gold: 2 } } } };
const run = (state: string, move: unknown, seat = 0): string =>
  engine.tryMove(state, JSON.stringify(move), seat);
const view = (state: string): View => JSON.parse(engine.webView(engine.stripSecret(state)));
const entries = (state: string) => journal(JSON.parse(engine.stripSecret(state)), view(state));

test('events expose engine rules and protections to spectators without revealing private cards', () => {
  const state = fixture('famine/epidemics');
  const catalog = view(state).eventCatalog!;
  assert.equal(new Set(catalog.map((e) => e.id)).size, catalog.length);
  const epidemic = catalog.find((e) => e.name === 'Epidemics')!;
  assert.ok(epidemic.rules.some((rule) => rule.includes('2 or more units')));
  assert.deepEqual(epidemic.targets, ['all']);
  assert.equal(epidemic.protectionAdvance, 'Sanitation');
  assert.ok(!epidemic.rules.includes('No base effect.'));
  const famine = catalog.find((e) => e.name === 'Famine')!;
  assert.ok(famine.rules.includes('Barbarians move.'));
  assert.deepEqual(famine.targets, ['active']);
  assert.equal(famine.protectionAdvance, 'Irrigation');
  assert.equal(view(state).objectiveCards.length, 0);
});

test('Epidemics explains a whiff for each civilization with fewer than two units', () => {
  const raw = JSON.parse(fixture('famine/epidemics'));
  raw.players[0].units = raw.players[0].units.slice(0, 1);
  raw.players[1].units = [];
  const state = run(JSON.stringify(raw), advance);
  const event = entries(state).find((e) => e.event)!;
  assert.equal(event.title, 'Epidemics');
  assert.deepEqual(
    event.event!.explanations.map((e) => [e.player, e.text]),
    [
      [0, '1 unit · requires 2+'],
      [1, '0 units · requires 2+'],
    ],
  );
  assert.equal(event.event!.pending, undefined);
  assert.equal(event.event!.outcomes.length, 0);
});

test('pending events never claim to have whiffed and casualties attach to the event after resolution', () => {
  let state = run(fixture('famine/epidemics'), advance);
  let event = entries(state).find((e) => e.event)!;
  assert.match(event.event!.pending!, /Waiting for Rome/);
  assert.equal(event.event!.explanations.length, 0);
  state = run(state, { Response: { SelectUnits: [7] } });
  const log = entries(state);
  event = log.find((e) => e.event)!;
  assert.equal(event.event!.pending, undefined);
  assert.equal(log.filter((e) => e.title === 'Epidemics').length, 1);
  assert.ok(
    event.event!.outcomes.some(
      (e) =>
        e.civilization === 'Rome' &&
        e.tokens.some((t) => t.icon === 'unit' && t.value === '−1' && t.label.includes('C2')),
    ),
  );
  assert.ok(
    event.event!.explanations.some((e) => e.player === 1 && e.text === '2 units · no loss recorded'),
    'The historical two-unit engine bug must not be presented as a legitimate exemption',
  );
});

test('Irrigation protection is explicit while barbarian side effects remain visible', () => {
  const state = run(fixture('famine/famine_protected'), advance);
  const event = entries(state).find((e) => e.event)!;
  assert.equal(event.title, 'Severe famine');
  assert.ok(
    event.event!.explanations.some((e) => e.player === 0 && e.protected && e.text.includes('Irrigation')),
  );
  assert.ok(event.event!.outcomes.some((e) => e.notes.some((n) => /Barbarian/i.test(n))));
  assert.ok(event.event!.info!.rules.includes('Barbarians move.'));
  assert.ok(!event.event!.explanations.some((e) => e.player === 1), 'Only the targeted player is protected');
});

const info: EventInfo = {
  id: 2,
  name: 'Epidemics',
  rules: ['Every player with 2 or more units loses a unit.'],
  baseEffect: null,
  protectionAdvance: 'Sanitation',
  protectionSpecialAdvance: null,
  targets: ['all'],
};
function history(actions: LoggedAction[]): Game {
  return {
    state: 'Playing',
    players: [
      { id: 0, civilization: 'Rome', units: [{ id: 0, unit_type: 'Settler', position: 'D2' }], advances: [] },
    ],
    map: { tiles: [] },
    current_player_index: 0,
    age: 1,
    round: 1,
    actions_left: 1,
    log_index: actions.length - 1,
    log: [{ age: 1, rounds: [{ round: 1, turns: [{ turn_type: { Player: 0 }, actions }] }] }],
  };
}
const publicView = (eventCatalog = [info]) => ({
  eventCatalog,
  players: [{ index: 0, civilizationAdvances: [] }] as unknown as View['players'],
});

test('later recruitment and research do not rewrite the explanation of earlier events', () => {
  const game = history([
    { log: ['A new game event has been triggered: Epidemics'] },
    {
      action: { Playing: { Recruit: {} } },
      log: ['Player1: Recruit: Gain 1 settler at D2'],
      items: [{ player: 0, Units: { units: { settlers: 1 }, balance: 'Gain' } }],
    },
    {
      action: { Playing: { Advance: {} } },
      log: ['Player1: Advance: Gain Sanitation without taking an event token'],
      items: [{ player: 0, Advance: { advance: 'Sanitation', balance: 'Gain' } }],
    },
  ]);
  game.players[0].units!.push({ id: 1, unit_type: 'Settler', position: 'D2' });
  game.players[0].advances = ['Sanitation'];
  const before = JSON.stringify(game);
  const event = journal(game, publicView()).find((e) => e.event)!;
  assert.deepEqual(event.event!.explanations, [{ player: 0, text: '1 unit · requires 2+' }]);
  assert.equal(JSON.stringify(game), before, 'Formatting does not mutate the game');
});

test('later loss of an advance does not erase historical protection', () => {
  const game = history([
    { log: ['A new game event has been triggered: Epidemics'] },
    {
      action: { Playing: 'EndTurn' },
      log: ['Player1: Anarchy: Lost Sanitation'],
      items: [{ player: 0, Advance: { advance: 'Sanitation', balance: 'Loss' } }],
    },
  ]);
  assert.equal(journal(game, publicView())[0].event!.explanations[0].text, 'Protected by Sanitation');
});

test('events with the same name use their recorded card ID rather than arbitrary rules', () => {
  const catalog = [9, 10].map((id) => ({ ...info, id, name: 'A good year', rules: [`Variant ${id}`] }));
  const game = history([
    {
      log: ['A new game event has been triggered: A good year', 'Player1: A good year: Gain 1 food'],
      items: [{ player: 0, origin: { Incident: 10 } }],
    },
  ]);
  assert.equal(journal(game, publicView(catalog))[0].event!.info?.id, 10);
  game.log![0].rounds[0].turns[0].actions![0].items = [];
  assert.equal(journal(game, publicView(catalog))[0].event!.info, undefined);
});

test('later activations of lasting effects remain at their own place in the journal', () => {
  const game = history([
    { log: ['A new game event has been triggered: Trojan Horse'] },
    {
      action: { Playing: { MoveUnits: {} } },
      log: [
        'Player1: Move: Pay 1 action',
        'Player1: Trojan Horse: Activated the Trojan Horse and gained 1 victory point',
      ],
    },
  ]);
  const log = journal(game);
  assert.equal(log.length, 3);
  assert.equal(log[0].event!.outcomes.length, 0);
  assert.match(log[2].text, /gained 1 victory point/);
});

test('named famine variants retain their full name and food loss, even after an actor rename', () => {
  const famine = { ...info, id: 52, name: 'Famine: Draught', protectionAdvance: 'Irrigation' };
  const game = history([
    {
      log: ['A new game event has been triggered: Famine: Draught', 'Old name: Famine: Draught: Lost 1 food'],
      items: [{ player: 0, origin: { Incident: 52 } }],
    },
  ]);
  const event = journal(game, publicView([famine]))[0];
  assert.equal(event.event!.outcomes[0].civilization, 'Rome');
  assert.deepEqual(
    event.event!.outcomes[0].tokens.map((t) => [t.icon, t.value]),
    [['food', '−1']],
  );
  assert.equal(event.event!.explanations.length, 0);
});

test('temporary Great Library protection is taken from that turn, not the current turn', () => {
  const game = history([
    { log: ['Player1: Great Library: Use Sanitation for the turn'] },
    { log: ['A new game event has been triggered: Epidemics'] },
  ]);
  assert.equal(journal(game, publicView())[1].event!.explanations[0].text, 'Protected by Sanitation');
  game.log![0].rounds[0].turns[0].actions!.reverse();
  assert.equal(journal(game, publicView())[0].event!.explanations[0].text, '1 unit · requires 2+');
});

test('Aqueduct protection uses its historical civilization prerequisite', () => {
  const famine = {
    ...info,
    name: 'Famine',
    targets: ['active'] as EventInfo['targets'],
    protectionAdvance: 'Irrigation',
    protectionSpecialAdvance: 'Aqueduct',
  };
  const game = history([{ log: ['A new game event has been triggered: Famine'] }]);
  const v = publicView([famine]);
  v.players[0].civilizationAdvances = [
    { id: 'Aqueduct', name: 'Aqueduct', prerequisites: [{ id: 'Engineering', name: 'Engineering' }] },
  ] as View['players'][number]['civilizationAdvances'];
  game.players[0].advances = ['Engineering'];
  assert.equal(journal(game, v)[0].event!.explanations[0].text, 'Protected by Aqueduct');
  game.players[0].advances = [];
  assert.equal(
    journal(game, v)[0].event!.explanations.some((e) => e.protected),
    false,
  );
});

test('city thresholds explain an ineligible civilization without mislabelling pending disasters', () => {
  const disaster = { ...info, name: 'Vulcan', protectionAdvance: null, minimumCities: 4 };
  const game = history([{ log: ['A new game event has been triggered: Vulcan'] }]);
  game.players[0].cities = [{ position: 'D2', mood_state: 'Happy' }];
  assert.equal(journal(game, publicView([disaster]))[0].event!.explanations[0].text, '1 city · requires 4+');
});
