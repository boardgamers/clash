import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { createTutorial } from '@boardgamers/protocol/tutorial';
import { chapters, createLesson } from './tutorial/lessons.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
for (const chapter of chapters) {
  test(`tutorial ${chapter.id}: complete walkthrough and atomic rejection`, async () => {
    const lesson = createLesson(
      chapter.id,
      engine,
      readFileSync(new URL(`./tutorial/positions/${chapter.id}.json`, import.meta.url), 'utf8'),
    );
    const saved = new Map<string, string>();
    const storage = {
      getItem: (key: string) => saved.get(key) ?? null,
      setItem: (key: string, value: string) => {
        saved.set(key, value);
      },
    };
    const controller = await createTutorial({ ...lesson, storage });
    for (let guard = 0; !controller.snapshot.completed && guard < 30; guard++) {
      const before = controller.snapshot.state;
      assert.equal(await controller.play({ step: 'unrelated', kind: 'move', move: 'Undo' }), false);
      assert.equal(controller.snapshot.state, before);
      const step = lesson.steps[controller.snapshot.step];
      if (controller.snapshot.canContinue) {
        await controller.continue();
        continue;
      }
      const offers = step.offers(before);
      assert.ok(offers.length, `No legal actions at ${chapter.id}/${step.id}`);
      let accepted = false;
      for (const offer of offers)
        if (await controller.play(offer.action)) {
          accepted = true;
          break;
        }
      assert.ok(accepted, `No accepted actions at ${chapter.id}/${step.id}: ${controller.snapshot.error}`);
    }
    assert.ok(controller.snapshot.completed);
    const final = JSON.parse(controller.snapshot.state.game);
    const initial = JSON.parse(lesson.initialState().game);
    if (chapter.id === 'first-turn') assert.equal(engine.currentPlayer(controller.snapshot.state.game), 1);
    if (chapter.id === 'research-paths') {
      assert.ok(final.players[0].advances.includes('Math'));
      assert.ok(final.players[0].advances.includes('Engineering'));
      assert.equal(final.actions_left, initial.actions_left - 2);
    }
    if (chapter.id === 'activation-happiness') {
      assert.equal(final.players[0].cities[0].mood_state, 'Neutral');
      assert.equal(final.players[0].cities[0].activations, 2);
      assert.equal(final.players[0].cities[1].mood_state, 'Neutral');
      assert.equal(final.players[0].cities[1].activations ?? 0, 0);
    }
    if (chapter.id === 'movement-founding') {
      assert.equal(
        final.actions_left,
        initial.actions_left - 2,
        'Both settlers share one Move action; founding spends one more action',
      );
      assert.ok(final.players[0].cities.some((city: any) => city.position === 'C2'));
    }
    if (chapter.id === 'sea-transport') {
      const view = JSON.parse(engine.webView(engine.stripSecret(controller.snapshot.state.game, 0), 0));
      const settler = view.units.find((unit: any) => unit.type === 'Settler');
      assert.equal(settler.position, 'C4');
      assert.notEqual(
        final.map.tiles.find(([position]: [string, string]) => position === 'C4')[1],
        'Unexplored',
      );
      assert.equal(final.actions_left, initial.actions_left - 2);
      assert.equal(settler.carrier, null);
      const ship = view.units.find((unit: any) => unit.type === 'Ship');
      assert.equal(ship.position, 'C3');
      assert.equal(
        final.map.tiles.find(([position]: [string, string]) => position === ship.position)[1],
        'Water',
        'Exploration must preserve water beneath the ship',
      );
      for (const [position, terrain] of initial.map.tiles) {
        if (terrain !== 'Unexplored')
          assert.equal(
            final.map.tiles.find(([p]: [string, string]) => p === position)[1],
            terrain,
            `Exploration preserves known terrain at ${position}`,
          );
      }
    }
    if (chapter.id === 'cultural-influence') {
      assert.equal(final.successful_cultural_influence, true);
      assert.equal(final.players[1].cities[0].city_pieces.temple, 0);
    }
    if (chapter.id === 'wonders-ownership') {
      assert.equal(final.actions_left, initial.actions_left - 1);
      assert.equal(final.players[0].cities[0].activations, 1);
      assert.equal(final.players[0].units.filter((unit: any) => unit.unit_type === 'Ship').length, 1);
    }
    if (chapter.id === 'incidents-hostiles') {
      assert.equal(engine.currentPlayer(controller.snapshot.state.game), 0);
      assert.deepEqual(
        final.players[0].cities.map((city: any) => city.mood_state),
        initial.players[0].cities.map((city: any) => city.mood_state),
      );
      assert.ok(
        final.players[1].cities.some(
          (city: any, index: number) => city.mood_state !== initial.players[1].cities[index].mood_state,
        ),
      );
    }
    if (chapter.id === 'objectives-ages') assert.equal(final.players[0].completed_objectives.length, 1);
    const restored = await createTutorial({ ...lesson, storage });
    assert.ok(restored.snapshot.completed, 'Validated saved history restores completion');
    assert.deepEqual(restored.snapshot.state, controller.snapshot.state);
    restored.destroy();
    await controller.restart();
    assert.equal(controller.snapshot.step, 0);
    assert.equal(controller.snapshot.state.game, lesson.initialState().game);
    controller.destroy();
  });
}

test('engine failure leaves tutorial game, completion and history unchanged', async () => {
  const initialGame = readFileSync(new URL('./tutorial/positions/first-turn.json', import.meta.url), 'utf8');
  const lesson = createLesson(
    'first-turn',
    {
      ...engine,
      tryMove: () => {
        throw new Error('Rejected by engine');
      },
    },
    initialGame,
  );
  const controller = await createTutorial(lesson);
  await controller.continue();
  const before = controller.snapshot.state;
  assert.equal(await controller.play(lesson.steps[controller.snapshot.step].offers(before)[0].action), false);
  assert.deepEqual(controller.snapshot.state, before);
  assert.equal(controller.snapshot.step, 1);
  assert.match(controller.snapshot.error, /Rejected by engine/);
  controller.destroy();
});

for (const id of ['first-turn', 'activation-happiness']) {
  test(`tutorial ${id}: accepts real-control choices and rejects invalid payments atomically`, async () => {
    const lesson = createLesson(
      id,
      engine,
      readFileSync(new URL(`./tutorial/positions/${id}.json`, import.meta.url), 'utf8'),
    );
    const controller = await createTutorial(lesson);
    while (controller.snapshot.canContinue) await controller.continue();
    const first = lesson.steps[controller.snapshot.step].offers(controller.snapshot.state)[0].action;
    assert.equal(first.kind, 'move');
    const action = structuredClone(first) as any;
    if (id === 'activation-happiness') {
      action.move.Playing.IncreaseHappiness.happiness_increases.reverse();
      action.move.Playing.IncreaseHappiness.payment = { mood_tokens: 0 };
      const before = controller.snapshot.state;
      assert.equal(await controller.play(action), false);
      assert.equal(controller.snapshot.state, before);
      action.move.Playing.IncreaseHappiness.payment = { mood_tokens: 2 };
    } else {
      const game = engine.stripSecret(controller.snapshot.state.game, 0);
      const view = JSON.parse(engine.webView(game, 0));
      const city = view.cities[0];
      action.move = JSON.parse(
        engine.webQuery(
          game,
          0,
          JSON.stringify({
            kind: 'collect',
            city: city.position,
            selections: [{ ...city.choices.find((choice: any) => choice.pile?.wood), times: 1 }],
            variant: 'Collect',
          }),
        ),
      ).action;
      assert.ok(action.move, 'Alternative collection is legal');
    }
    assert.equal(await controller.play(action), true, controller.snapshot.error);
    if (id === 'first-turn') {
      const research = structuredClone(
        lesson.steps[controller.snapshot.step].offers(controller.snapshot.state)[0].action,
      ) as any;
      research.move.Playing.Advance.payment = { gold: 2, ideas: 0 };
      assert.equal(await controller.play(research), true, controller.snapshot.error);
    }
    controller.destroy();
  });
}
