import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const fixture = (name: string) =>
  JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
const move = (state: string, action: unknown) => engine.tryMove(state, JSON.stringify(action), 0);
const view = (state: string, seat: number | undefined = 0) =>
  JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));

// Great Architect (155) and Great Diplomat (157) are public; Great Scientist (122)
// is a private Action card and must still count. Spy's two printed copies are 7/8.
const publicCards = new Set([155, 157]);
const counts = [
  { action: 1, objective: 1, wonder: 1 },
  { action: 3, objective: 2, wonder: 0 },
  { action: 0, objective: 0, wonder: 1 },
  { action: 2, objective: 1, wonder: 2 },
];
async function spyGame(players: number, spy: number) {
  const game = JSON.parse(await engine.init(players, [], {}, 'spy-target-counts', {}));
  game.current_player_index = 0;
  const hands = [
    { action_cards: [spy, 1], objective_cards: [1], wonder_cards: ['GreatWall'] },
    { action_cards: [2, 3, 122, 155], objective_cards: [2, 3], wonder_cards: [] },
    { action_cards: [157], objective_cards: [], wonder_cards: ['Pyramids'] },
    { action_cards: [5, 6], objective_cards: [4], wonder_cards: ['GreatGardens', 'GreatLibrary'] },
  ];
  for (let i = 0; i < players; i++) {
    Object.assign(game.players[i], hands[i]);
    game.players[i].civilization = ['Rome', 'Greece', 'Egypt', 'China'][i];
  }
  game.players[0].resources.culture_tokens = 3;
  return game;
}
function targetState(game: unknown, spy: number) {
  const payment = move(JSON.stringify(game), { Playing: { ActionCard: spy } });
  assert.ok(view(payment).decision.options.every((option: any) => !Object.hasOwn(option, 'handCounts')));
  return move(payment, { Response: { Payment: [{ culture_tokens: 1 }] } });
}
function assertPrivateDecision(state: string, players: number) {
  for (let seat = 1; seat < players; seat++) {
    assert.equal(view(state, seat).decision, null, 'opponents must not receive Spy choices');
    assert.equal(view(state, seat).choiceDecision, null);
  }
  const spectator = JSON.parse(engine.webView(engine.stripSecret(state, undefined), undefined));
  assert.ok(spectator.decision == null, 'spectators must not receive Spy choices');
  assert.ok(spectator.choiceDecision == null);
}

for (const players of [3, 4]) {
  for (const spy of [7, 8]) {
    test(`${players}-player Spy ${spy} exposes only redaction-safe target hand counts, then ordinary swap choices`, async () => {
      const game = await spyGame(players, spy);
      const state = targetState(game, spy);
      const authoritative = JSON.parse(engine.webView(state, 0)).decision;
      const redacted = JSON.parse(engine.stripSecret(state, 0));
      const decision = view(state).decision;
      assert.equal(decision.name, 'Spy');
      assert.deepEqual(decision.origin, { CivilCard: spy });
      assert.deepEqual([decision.min, decision.max], [1, 1]);
      assert.deepEqual(
        decision.options.map((option: any) => option.value),
        players === 3 ? [1, 2] : [1, 2, 3],
      );
      assert.deepEqual(
        decision.options,
        authoritative.options,
        'counts must survive stripping private identities',
      );
      assert.deepEqual(
        redacted.players[0].secrets ?? [],
        [],
        'no hand has been revealed before choosing a target',
      );

      for (const option of decision.options) {
        const player = game.players[option.value];
        // Exact option shape rules out card identities, rules, objectives, and owner/card details.
        assert.deepEqual(option, {
          value: player.id,
          name: player.civilization,
          description: '',
          position: null,
          handCounts: counts[player.id],
        });
        const hidden = redacted.players[player.id];
        assert.deepEqual(
          hidden.action_cards ?? [],
          player.action_cards.map((id: number) => (publicCards.has(id) ? id : 0)),
        );
        assert.deepEqual(
          hidden.objective_cards ?? [],
          player.objective_cards.map(() => 0),
        );
        assert.deepEqual(
          hidden.wonder_cards ?? [],
          player.wonder_cards.map(() => 'Hidden'),
        );

        const query = JSON.parse(
          engine.webQuery(
            engine.stripSecret(state, 0),
            0,
            JSON.stringify({ kind: 'decision', values: [player.id], payments: [] }),
          ),
        );
        assert.deepEqual(query.action, { Response: { SelectPlayer: player.id } });
        const swap = move(state, query.action);
        const swapDecision = view(swap).decision;
        assert.match(swapDecision.description, /card to swap/);
        assert.equal(JSON.parse(swap).events.at(-1).event_type.ActionCard.selected_player, player.id);
        assert.ok(swapDecision.options.length > 0);
        for (const choice of swapDecision.options) {
          assert.ok(!Object.hasOwn(choice, 'handCounts'), 'counts belong only to the target picker');
          assert.ok(!publicCards.has(choice.value.ActionCard), 'public Great Persons cannot be swapped');
        }
        for (const [kind, key] of [
          ['action', 'ActionCard'],
          ['objective', 'ObjectiveCard'],
          ['wonder', 'Wonder'],
        ] as const) {
          assert.equal(
            swapDecision.options.filter(
              (choice: any) => choice.owner?.player === player.id && key in choice.value,
            ).length,
            option.handCounts[kind],
            'target count must match the cards actually available to swap',
          );
        }
        assertPrivateDecision(swap, players);
      }
      assertPrivateDecision(state, players);
    });
  }
}

test('Spy does not offer an empty hand or a hand containing only a public Great Person', async () => {
  for (const publicOnly of [[], [157]]) {
    const game = await spyGame(4, 7);
    game.players[2].action_cards = [];
    Object.assign(game.players[3], { action_cards: publicOnly, objective_cards: [], wonder_cards: [] });
    const decision = view(targetState(game, 7)).decision;
    assert.deepEqual(
      decision.options.map((option: any) => option.value),
      [1, 2],
    );
    assert.deepEqual(
      decision.options.map((option: any) => option.handCounts),
      counts.slice(1, 3),
    );
  }
});

test('unrelated SelectPlayer decisions do not acquire Spy hand counts', async () => {
  const game = fixture('status_phase/determine_first_player');
  game.players.push(...JSON.parse(await engine.init(2, [], {}, 'spy-unrelated', {})).players.slice(2));
  const state = JSON.stringify(game);
  assert.ok(game.events.at(-1).handler.request.SelectPlayer);
  for (const decision of [view(state).decision, JSON.parse(engine.webView(state, 0)).decision]) {
    assert.equal(decision.options.length, 2);
    for (const option of decision.options) assert.ok(!Object.hasOwn(option, 'handCounts'));
  }
});

test('Spies identifies each redacted-hand owner and uses that owner for objective progress', async () => {
  const game = fixture('action_cards/spy');
  game.players[0].objective_cards = [1];
  game.players[1].objective_cards = [2];
  game.players.push(...JSON.parse(await engine.init(2, [], {}, 'spy', {})).players.slice(2));
  let state = move(JSON.stringify(game), { Playing: { ActionCard: 7 } });
  state = move(state, { Response: { Payment: [{ culture_tokens: 1 }] } });
  for (const option of view(state).decision.options) {
    const owner = option.value.ActionCard === 1 || option.value.ObjectiveCard === 1 ? 0 : 1;
    assert.equal(option.owner.player, owner);
    assert.equal(option.owner.you, owner === 0);
    if (option.value.ObjectiveCard) {
      const card = view(state, owner).objectiveCards.find((c: any) => c.id === option.value.ObjectiveCard);
      assert.deepEqual(
        option.card.objectives.map((o: any) => o.progress),
        card.objectives.map((o: any) => o.progress),
      );
      assert.deepEqual(
        option.card.objectives.map((o: any) => o.conditionMet),
        card.objectives.map((o: any) => o.conditionMet),
      );
    }
  }
  assertPrivateDecision(state, 2);
});
