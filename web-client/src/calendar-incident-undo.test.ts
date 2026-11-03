import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fingerprint, predictMove, predictUndo } from './prediction.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (state: string, action: unknown, seat = engine.currentPlayer(state)): string =>
  engine.tryMove(state, JSON.stringify(action), seat);
const strip = (state: string, seat: number | undefined = 0): string => engine.stripSecret(state, seat);
const view = (state: string, seat: number | undefined = 0) =>
  JSON.parse(engine.webView(strip(state, seat), seat));
const research = (advance: string) => ({ Playing: { Advance: { advance, payment: { food: 2 } } } });

async function ready(calendar = true) {
  const game = JSON.parse(await engine.init(2, [], {}, 'calendar-incident-undo', {}));
  game.current_player_index = 0;
  game.board_history = { id: 'calendar-incident-undo', frames: [] };
  for (let i = 0; i < 2; i++) {
    game.players[i].civilization = ['Maya', 'Rome'][i];
    game.players[i].action_cards = [];
    game.players[i].objective_cards = [];
    game.players[i].resources = { food: 7, ideas: 7, culture_tokens: 3, mood_tokens: 3 };
  }
  game.players[0].advances = ['Farming', 'Mining', 'Math', ...(calendar ? ['Astronomy'] : [])];
  game.players[0].incident_tokens = 1;
  // Vulcan/Earthquake have no effect with a single city.
  game.incidents_left = [29, 30];
  game.time_increments = [8, 5];
  return JSON.stringify(game);
}

test('Calendar exposes Undo for the owner after a known incident, without predicting the hidden draw', async () => {
  const before = await ready();
  const action = research('Storage');
  assert.deepEqual(predictMove(engine, strip(before), action, 0), { reason: 'hidden' });
  const after = move(before, action);
  assert.equal(view(after).canUndo, true);
  assert.equal(view(after, 1).canUndo, false);
  assert.equal(JSON.parse(engine.webView(engine.stripSecret(after, undefined), undefined)).canUndo, false);
  assert.deepEqual(engine.timeIncrements(after), [8, 5]);
  const undone = move(after, 'Undo');
  const predicted = predictUndo(strip(before), strip(after));
  assert.ok(predicted);
  assert.equal(fingerprint(predicted), fingerprint(strip(undone)));
  assert.equal(view(undone).canUndo, false);
  assert.equal(view(undone).canRedo, true);
  assert.deepEqual(JSON.parse(undone).incidents_left, [29, 30]);
  // Redo still waits for the server's deck, rather than revealing the masked order.
  assert.deepEqual(predictMove(engine, strip(undone), 'Redo', 0), { reason: 'hidden' });
  const redone = move(undone, 'Redo');
  assert.equal(fingerprint(strip(redone)), fingerprint(strip(after)));
});

test('predicted Undo preserves the new knowledge floor and cannot refund Astronomy', async () => {
  const game = JSON.parse(await ready(false));
  game.players[0].incident_tokens = 2;
  const astronomy = move(JSON.stringify(game), research('Astronomy'));
  assert.equal(view(astronomy).canUndo, true);
  const after = move(astronomy, research('Storage'));
  assert.equal(view(after).canUndo, true);
  const undone = move(after, 'Undo');
  const predicted = predictUndo(strip(astronomy), strip(after));
  assert.ok(predicted);
  assert.equal(fingerprint(predicted), fingerprint(strip(undone)));
  assert.equal(view(undone).canUndo, false);
  assert.throws(() => move(undone, 'Undo'));
  assert.ok(JSON.parse(undone).players[0].advances.includes('Astronomy'));
});

test('Calendar incident Undo does not bypass acquiring Calendar or other card draws', async () => {
  for (const [before, advance] of [
    [await ready(false), 'Storage'],
    [await ready(false), 'Astronomy'],
    [await ready(), 'Writing'],
  ] as const) {
    const after = move(before, research(advance));
    assert.equal(view(after).canUndo, false, advance);
    assert.throws(() => move(after, 'Undo'));
    assert.deepEqual(engine.timeIncrements(after), [9, 5]);
  }
});
