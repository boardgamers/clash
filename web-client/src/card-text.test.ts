import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { handCardDescription } from './card-text.ts';
import { readFile } from 'node:fs/promises';
import { createTranslator, languages } from './localization/runtime.js';
import { localizedParts } from './localized-parts.ts';
import { journalParts } from './model.ts';
import type { View } from './types.ts';

test('held Elder Statesman shows its playable effect and ordinary cards retain all wording', async () => {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const game = JSON.parse(await engine.init(2, [], {}, 'great-people-hand', {}));
  game.players[0].action_cards = [123, 157, 29];
  const cards = JSON.parse(engine.webView(JSON.stringify(game), 0)).actionCards;
  const elder = cards.find((card: { name: string }) => card.name === 'Elder Statesman');
  assert.ok(elder);
  const diplomat = cards.find((card: { name: string }) => card.name === 'Great Diplomat');
  assert.equal(diplomat, undefined, 'the public treaty is not a playable hand card');
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

// These are the restrictions in server/src/content/tactics_cards.rs, including
// unrestricted cards. Read the actual webView descriptions rather than testing
// only hand-authored catalog fragments: the server appends the punctuation.
test('all engine tactics descriptions localize their restrictions and complete effects', async () => {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const state = await engine.init(2, [], {}, 'tactics-localization', {});
  const view = JSON.parse(engine.webView(state, 0)) as View;
  const tactics = new Map(view.cardCatalog!.flatMap((card) => card.tactics ? [[card.tactics.name, card.tactics] as const] : []));
  const army = 'Requires your Army in the battle';
  const restrictions: Record<string, string[]> = {
    Peltasts: [army],
    Encircled: [army],
    'Wedge Formation': ['Attacking only', army],
    'High Morale': [],
    'Heavy Resistance': ['Defending only'],
    'High Ground': ['Land battle'],
    Surprise: [],
    Siege: ['Attacking only', 'City battle', army],
    'For the People': ['Defending only', 'City battle'],
    'Improved Defenses': ['Defending only', 'City battle'],
    'Tactical Retreat': ['Defending only', army],
    'Defensive Formation': ['Defending only'],
    Scout: [],
    Martyr: ['Requires your Army or Ship in the battle'],
    Archers: [army],
    Flanking: ['Attacking only'],
  };
  assert.deepEqual([...tactics.keys()].sort(), Object.keys(restrictions).sort());
  for (const locale of Object.keys(languages)) {
    const catalog: Record<string, string> = JSON.parse(await readFile(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    const requirementLabels: Record<string, string> = {
      'Attacking only': catalog['Attacking only'],
      'Defending only': catalog['Defending only'],
      'Land battle': catalog['{p0} battle'].replace('{p0}', catalog.Land),
      'City battle': catalog['{p0} battle'].replace('{p0}', catalog.City),
      [army]: catalog['Requires your {p0} in the battle'].replace('{p0}', catalog.Army),
      'Requires your Army or Ship in the battle': catalog['Requires your {p0} in the battle']
        .replace('{p0}', `${catalog.Army} ${catalog.or} ${catalog.Ship}`),
    };
    for (const [name, requirements] of Object.entries(restrictions)) {
      const card = tactics.get(name)!;
      const prefix = requirements.length ? requirements.join(' · ') + '. ' : '';
      assert.ok(card.description.startsWith(prefix), `${name}: engine restriction prefix`);
      const effect = card.description.slice(prefix.length);
      assert.ok(Object.hasOwn(catalog, effect), `${locale}: complete ${name} effect must exist`);
      const expected = (requirements.length ? requirements.map((part) => requirementLabels[part]).join(' · ') + '. ' : '') + catalog[effect];
      // ResourceText uses this exact path before restoring its resource icons.
      const parts = localizedParts(card.description, journalParts(card.description), translator.translate);
      assert.equal(parts.map((part) => part.text).join(''), expected, `${locale}: ${name}`);
      assert.equal(translator.translate(card.description), expected, `${locale}: ${name} cached`);
      if (locale === 'fr') assert.doesNotMatch(expected, /\byour\b|\bor\b|\bArmy\b|\bShip\b|\bRequires\b/);
      if (locale === 'fr' && name === 'Siege') {
        assert.deepEqual(parts.filter((part) => 'resource' in part), [{ text: '2 Nourriture', resource: 'food' }]);
      }
    }
    if (locale === 'fr') {
      assert.equal(
        translator.translate(tactics.get('Wedge Formation')!.description),
        "En attaque seulement · Nécessite votre Armée dans la bataille. Recevez 1 en valeur de combat pour chaque unité d'armée en défense",
      );
      assert.equal(
        translator.translate(tactics.get('Martyr')!.description),
        'Nécessite votre Armée ou Navire dans la bataille. Retirez 1 unité avant le lancer de dé : votre adversaire doit retirer 1 unité. Ignorez la carte de tactique ennemie.',
      );
    }
  }
});
