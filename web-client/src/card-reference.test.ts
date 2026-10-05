import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { cardReferences } from './card-reference.ts';
import { namedTextParts } from './research-links.ts';
import type { View } from './types.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
test('card rules catalog is identical for players and spectators, with no playable moves or private hand', async () => {
  const state = await engine.init(2, [], { civilization: 'Random' }, 'card-reference-rules', {});
  const views = [0, 1, undefined].map(
    (seat) => JSON.parse(engine.webView(engine.stripSecret(state, seat), seat)) as View,
  );
  const catalog = views[0].cardCatalog!;
  assert.ok(catalog.length > 40);
  for (const view of views) assert.deepEqual(view.cardCatalog, catalog);
  assert.ok(catalog.every((c) => !('action' in c) && !('reason' in c)));
  assert.match(
    catalog.find((c) => c.name === 'Great Prophet')!.description,
    /paying its normal resource cost/,
  );
  const cards = cardReferences(views[0]);
  const text = 'Played Great Prophet, then Mass Production. NotGreat Prophet stays plain.';
  const parts = namedTextParts(text, cards);
  assert.equal(parts.map((p) => p.text).join(''), text);
  assert.deepEqual(
    parts.filter((p) => p.reference).map((p) => p.reference!.card.name),
    ['Great Prophet', 'Mass Production'],
  );
  const battle = cards.find((c) => c.card.tactics && c.name === c.card.tactics.name && c.name !== c.card.name)!;
  assert.ok(battle);
  assert.equal(namedTextParts(battle.name, cards)[0].reference!.card.id, battle.card.id);
});
