import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { printedCardName } from './card-names.ts';
import { createTranslator, languages } from './localization/runtime.js';
import type { View } from './types.ts';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const rule =
  'Immediately after a battle where you were attacked and eliminated at least 2 army units (including barbarian army units).';

test('both printed Defiance cards show the defender casualty condition, not a victory condition', async () => {
  const game = JSON.parse(await engine.init(2, [], {}, 'defiance-card-text', {}));
  game.players[0].objective_cards = [27, 28, 7];
  const view: View = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  for (const id of [27, 28]) {
    const card = view.objectiveCards.find((card) => card.id === id)!;
    assert.ok(card, `held objective card ${id}`);
    const defiance = card.objectives.find((objective) => objective.name === 'Resistance')!;
    assert.ok(defiance);
    assert.equal(printedCardName(defiance.name, 'objective'), 'Defiance');
    assert.equal(defiance.timing, 'Instant');
    assert.equal(defiance.description, rule);
    assert.doesNotMatch(defiance.description, /won|win|victory/i);
  }
  // The legacy engine name "Defiance" belongs to a different printed objective.
  // Its victory requirement must not be changed by the Resistance/Defiance fix.
  const againstAllOdds = view.objectiveCards
    .find((card) => card.id === 7)!
    .objectives.find((objective) => objective.name === 'Defiance')!;
  assert.equal(printedCardName(againstAllOdds.name, 'objective'), 'Against All Odds');
  assert.equal(
    againstAllOdds.description,
    'You won a battle against a human player despite having fewer units than them.',
  );
});

test('the engine offers and persists one Defiance claim after an actual defensive loss or draw', async () => {
  const npcs = JSON.parse(await engine.init(2, [], {}, 'defiance-battles', {})).players.slice(2);
  for (const card of [27, 28]) {
    for (const attackerCount of [2, 3]) {
      const game = JSON.parse(
        await readFile(
          new URL('../../server/tests/test_games/tactics_cards/heavy_resistance.json', import.meta.url),
          'utf8',
        ),
      );
      for (const player of game.players) {
        player.objective_cards = [];
        player.action_cards = [];
        player.advances = ['Farming', 'Mining', 'Tactics'];
      }
      for (const npc of npcs) {
        if (!game.players.some((player: any) => player.civilization === npc.civilization)) {
          game.players.push({ ...npc, id: game.players.length });
        }
      }
      game.players[1].objective_cards = [27, 28];
      const units = Array.from({ length: attackerCount }, (_, id) => id);
      game.players[0].units = units.map((id) => ({ id, unit_type: 'Infantry', position: 'C2' }));
      game.players[0].next_unit_id = attackerCount;
      game.players[1].cities.push(
        ...['B1', 'B3'].map((position) => ({ position, mood_state: 'Neutral' })),
      );
      game.dice_roll_outcomes = Array(attackerCount + 2).fill(11);
      let state = engine.tryMove(
        JSON.stringify(game),
        JSON.stringify({ Movement: { Move: { units, destination: 'C1', payment: {} } } }),
        0,
      );
      const afterBattle = JSON.parse(state);
      const stats = afterBattle.log
        .flatMap((age: any) => age.rounds ?? [])
        .flatMap((round: any) => round.turns ?? [])
        .flatMap((turn: any) => turn.actions ?? [])
        .find((action: any) => action.combat_stats)?.combat_stats;
      assert.equal(stats.result, attackerCount === 3 ? 'AttackerWins' : 'Draw');
      assert.deepEqual(stats.attacker.losses, { infantry: 2 });
      assert.deepEqual(stats.defender.losses, { infantry: 2 });
      assert.equal(engine.currentPlayer(state), 1);
      if (attackerCount === 3) {
        const view: View = JSON.parse(engine.webView(engine.stripSecret(state, 1), 1));
        assert.equal(view.decision?.name, 'Place Settler');
        assert.ok(!afterBattle.events.some((event: any) => event.event_type.SelectObjectives));
        state = engine.tryMove(state, JSON.stringify({ Response: { SelectPositions: ['B1'] } }), 1);
      }
      const pending = JSON.parse(state);
      const objective = pending.events.find((event: any) => event.event_type.SelectObjectives);
      assert.equal(objective.event_type.SelectObjectives.shown_objective, 'Resistance');
      assert.deepEqual(objective.handler.request.SelectHandCards.choices, [
        { ObjectiveCard: 27 },
        { ObjectiveCard: 28 },
      ]);
      // The WASM API reloads this serialized pending choice before claiming it.
      const claimed = JSON.parse(
        engine.tryMove(
          JSON.stringify(pending),
          JSON.stringify({ Response: { SelectHandCards: [{ ObjectiveCard: card }] } }),
          1,
        ),
      );
      assert.equal(claimed.players[1].completed_objectives.length, 1);
      assert.equal(claimed.players[1].completed_objectives[0].name, 'Resistance');
      assert.equal(claimed.players[1].completed_objectives[0].card, card);
      assert.deepEqual(claimed.players[1].objective_cards, [card === 27 ? 28 : 27]);
      assert.equal(claimed.events?.length ?? 0, 0, 'the second copy is not offered again');
    }
  }
});

test('the corrected Defiance rule is translated in every viewer locale', async () => {
  for (const locale of Object.keys(languages)) {
    const catalog = JSON.parse(
      await readFile(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'),
    );
    assert.ok(Object.hasOwn(catalog, rule), `${locale}: corrected rule exists`);
    const translator = createTranslator({ [locale]: catalog }, locale);
    assert.equal(translator.translate(rule), catalog[rule]);
    assert.ok(catalog[rule].length > 0);
    if (locale !== 'en') assert.notEqual(catalog[rule], rule, `${locale}: no English fallback`);
    assert.equal(
      Object.hasOwn(catalog, 'You won a battle as defender and killed at least 2 enemy units.'),
      false,
      `${locale}: obsolete victory rule removed`,
    );
  }
});
