import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { handCardDescription } from './card-text.ts';

test('held Elder Statesman shows its playable effect and ordinary cards retain all wording', async () => {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const game = JSON.parse(await engine.init(2, [], {}, 'great-people-hand', {}));
  game.players[0].action_cards = [123, 29];
  const cards = JSON.parse(engine.webView(JSON.stringify(game), 0)).actionCards;
  const elder = cards.find((card: { name: string }) => card.name === 'Elder Statesman');
  assert.ok(elder);
  for (const card of cards) {
    const [prefix, effect] = card.description.split('Action card: ');
    if (!effect) {
      assert.equal(handCardDescription(card.description), card.description);
      continue;
    }
    assert.match(prefix, /take the Event Card/);
    assert.equal(handCardDescription(card.description), effect.trim(), card.name);
  }
  assert.match(handCardDescription(elder.description), /^You may advance in any Democracy/);
  assert.equal(
    handCardDescription('Collect resources from 2 extra tiles.'),
    'Collect resources from 2 extra tiles.',
  );
});
