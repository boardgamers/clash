import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTranslator, resolveLocale } from './localization/runtime.js';

test('resolves regional BGS language preferences', () => {
  assert.equal(resolveLocale('pt-PT'), 'pt-BR');
  assert.equal(resolveLocale('zh-Hant-HK'), 'zh-TW');
  assert.equal(resolveLocale('zh-Hans-SG'), 'zh-CN');
  assert.equal(resolveLocale('zh'), 'zh-CN');
  assert.equal(resolveLocale('ZH_cn'), 'zh-CN');
  assert.equal(resolveLocale('fr_CA'), 'fr');
  assert.equal(resolveLocale('unknown'), 'en');
});

test('translates parameterized rules without translating player names', () => {
  const translator = createTranslator(
    { fr: { '{p0} gained {p1} food.': '{p0} a gagné {p1} nourriture.', Happy: 'Heureuse' } },
    'fr',
  );
  translator.setNames(['Happy']);
  assert.equal(translator.translate('Happy gained 2 food.'), 'Happy a gagné 2 nourriture.');
  assert.equal(translator.translate('Happy'), 'Happy');
});

test('switching locales clears cached translations and preserves whitespace', () => {
  const translator = createTranslator(
    { fr: { 'End turn': 'Fin du tour' }, de: { 'End turn': 'Zug beenden' } },
    'fr',
  );
  assert.equal(translator.translate(' End turn\n'), ' Fin du tour\n');
  translator.setLocale('de');
  assert.equal(translator.translate(' End turn\n'), ' Zug beenden\n');
  translator.setLocale('en');
  assert.equal(translator.translate(' End turn\n'), ' End turn\n');
});

import { createCatalogLoader } from './localization/loader.js';

test('English downloads nothing; concurrent French loads fetch only French once', async () => {
  const calls: string[] = [];
  const loader = createCatalogLoader(
    { './fr.json': '/fr.json', './de.json': '/de.json' },
    async (url: string) => {
      calls.push(url);
      return { ok: true, json: async () => ({ Research: 'Recherche' }) };
    },
  );
  await loader.load('en');
  assert.deepEqual(calls, []);
  assert.deepEqual(await Promise.all([loader.load('fr'), loader.load('fr-CA')]), ['fr', 'fr']);
  assert.deepEqual(calls, ['/fr.json']);
  await loader.load('fr');
  assert.deepEqual(calls, ['/fr.json']);
  assert.deepEqual(loader.catalogs.fr, { Research: 'Recherche' });
});

test('a failed catalog is retryable and never cached as a success', async () => {
  let attempts = 0;
  const loader = createCatalogLoader({ './fr.json': '/fr.json' }, async () => {
    attempts++;
    return attempts === 1
      ? { ok: false, status: 503 }
      : { ok: true, json: async () => ({ Research: 'Recherche' }) };
  });
  await assert.rejects(loader.load('fr'), /503/);
  assert.equal(loader.catalogs.fr, undefined);
  assert.equal(await loader.load('fr'), 'fr');
  assert.equal(attempts, 2);
});

test('malformed catalog contents are rejected', async () => {
  const loader = createCatalogLoader({ './fr.json': '/fr.json' }, async () => ({
    ok: true,
    json: async () => ({ Research: 42 }),
  }));
  await assert.rejects(loader.load('fr'), /Invalid language catalog/);
  assert.equal(loader.catalogs.fr, undefined);
});

import { mountLocalization } from './localization/runtime.js';

test('split Svelte text preserves live values through translation and locale switches', () => {
  let notify: (records: unknown[]) => void = () => {};
  const originalObserver = globalThis.MutationObserver;
  globalThis.MutationObserver = class {
    constructor(callback: (records: unknown[]) => void) {
      notify = callback;
    }
    observe() {}
    disconnect() {}
  } as unknown as typeof MutationObserver;
  const attributes = new Map<string, string>();
  const element: any = {
    nodeType: 1,
    childNodes: [],
    getAttribute: (key: string) => attributes.get(key) ?? null,
    setAttribute: (key: string, value: string) => attributes.set(key, value),
    removeAttribute: (key: string) => attributes.delete(key),
    hasAttribute: (key: string) => attributes.has(key),
    matches: () => false,
    closest: () => null,
  };
  element.childNodes = ['You have ', '3', ' unused ', 'actions', '.'].map((data) => ({
    nodeType: 3,
    data,
    parentElement: element,
  }));
  try {
    const localization = mountLocalization(
      element,
      { fr: { 'You have {p0} unused {p1}.': 'Il reste {p0} {p1}.', actions: 'actions', action: 'action' } },
      'fr',
    );
    const text = () => element.childNodes.map((node: any) => node.data).join('');
    assert.equal(text(), 'Il reste 3 actions.');
    // Svelte updates only its number and plural nodes, leaving static nodes alone.
    element.childNodes[1].data = '1';
    element.childNodes[3].data = 'action';
    notify([{ type: 'characterData', target: element.childNodes[1] }]);
    assert.equal(text(), 'Il reste 1 action.');
    localization.setLocale('en');
    assert.equal(text(), 'You have 1 unused action.');
    localization.setLocale('fr');
    assert.equal(text(), 'Il reste 1 action.');
    localization.destroy();
  } finally {
    globalThis.MutationObserver = originalObserver;
  }
});

test('translates numeric threshold captions without dropping the plus sign', () => {
  const translator = createTranslator({ fr: { 'Roll total · need': 'Résultat total · requis' } }, 'fr');
  assert.equal(translator.translate('Roll total · need 5+'), 'Résultat total · requis 5+');
});

test('translates resource quantities within payment labels', () => {
  const translator = createTranslator({ fr: { 'Pay {p0}': 'Payer {p0}', mood: 'humeur' } }, 'fr');
  assert.equal(translator.translate('Pay 1 mood'), 'Payer 1 humeur');
  assert.equal(translator.translate('1–3 mood'), '1–3 humeur');
});

// Exercise shipped catalogs, not only small synthetic examples: broad historical
// templates must not swallow current complete rules or optional UI suffixes.
test('real catalogs translate current rules and optional caption variants in every language', async () => {
  const { readFile } = await import('node:fs/promises');
  const { languages } = await import('./localization/runtime.js');
  const { rustStrings, rustTemplate } = await import('../scripts/localization-source.mjs');
  const rules = [];
  for (const [file, prefix] of [
    ['content/action_cards/spy.rs', "Look at another player's hand"],
    ['content/objectives/unit_objectives.rs', 'At least 4 of your cities each contain'],
    ['content/objectives/unit_objectives.rs', 'You own a city at least 5 spaces'],
    ['content/objectives/unit_objectives.rs', 'You could form at least 3 trade routes.'],
    ['content/objectives/unit_objectives.rs', 'You could form at least 2 trade routes using your ships.'],
    ['content/action_cards/mercenaries.rs', 'You may move any number of Barbarian armies'],
    ['leader_ability.rs', 'A: When drawing a wonder:'],
  ]) {
    const source = await readFile(new URL(`../../server/src/${file}`, import.meta.url), 'utf8');
    const rule = rustStrings(source).find(({ value }: { value: string }) => value.startsWith(prefix));
    assert.ok(rule, file);
    rules.push(rustTemplate(rule.value).replace(/\s+/g, ' ').trim());
  }
  const examples: [string, string[]][] = [
    ['Build {p0}', ['Temple']],
    ['Build {p0} at {p1}', ['Temple', 'C2']],
    ['Continue to payment at {p0}', ['C2']],
    ['Free with {p0}', ['Engineering']],
    ['Free with {p0} · once per turn', ['Engineering']],
    ['city size {p0}', ['3']],
    ['city size {p0}, Fortress', ['3']],
    ['{p0} army ({p1} aboard)', ['2', '1']],
    ['Recruit {p0} unit', ['1']],
    ['Recruit {p0} units & action card', ['2']],
    ['{p0}: gained {p1} at {p2}', ['Alice', '2', 'C2']],
  ];
  for (const locale of Object.keys(languages)) {
    const catalog = JSON.parse(await readFile(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    for (const key of [
      'Clash expects a serialized game state.', 'Missing building: market', 'Illegal Nomads move',
      'Unknown preview', 'Invalid player', 'Analysis requires a seed and a valid requesting seat',
      'Too many unexplored map blocks', 'Known cards exceed hand size',
      'Visible card counts exceed the card catalogue', 'Invalid replay history cursor',
      'Invalid replay player count', 'Invalid replay player indices',
      'Replay target {p0} is outside the action history', 'Replay history is missing a setup civilization',
      'Replay history is missing a draft civilization', 'Replay target {p0} is not a complete action boundary',
      'Invalid replay options: {p0}', 'Public map does not match the standard tile catalogue',
    ]) {
      assert.ok(Object.hasOwn(catalog, key), `${locale}: missing error ${key}`);
      if (locale !== 'en') assert.notEqual(catalog[key], key, `${locale}: untranslated error ${key}`);
    }
    translator.setNames(['Alice']);
    const format = (text: string, values: string[]) => text.replace(/\{p(\d+)\}/g, (_, i) => values[Number(i)]);
    for (const rule of rules) {
      assert.ok(Object.hasOwn(catalog, rule), `${locale}: missing current engine rule ${rule}`);
      const values = ['Great Library', 'Great Library', 'Great Library'];
      const rendered = format(rule, values);
      assert.equal(translator.translate(rendered), format(catalog[rule], values.map((value) => translator.translate(value))), `${locale}: ${rendered}`);
      if (locale !== 'en') assert.notEqual(catalog[rule], rule, `${locale}: untranslated rule`);
    }
    for (const [key, values] of examples) {
      assert.ok(Object.hasOwn(catalog, key), `${locale}: missing ${key}`);
      assert.equal(translator.translate(format(key, values)), format(catalog[key], values.map((value) => translator.translate(value))), `${locale}: ${key}`);
    }
    const segments = ['Forest', 'Rome', 'city size 3, Fortress', 'Happy', '2 army (1 aboard)', '1 ships'];
    assert.equal(translator.translate(segments.join(' · ')), segments.map((segment) => translator.translate(segment)).join(' · '), `${locale}: composed strategy tooltip`);
  }
});

test('tactics requirement punctuation cannot fall through to broad sentence templates', () => {
  const translator = createTranslator({ fr: {
    'Requires your {p0} in the battle': 'Nécessite votre {p0} dans la bataille',
    'Requires {p0}': 'Nécessite {p0}',
    '{p0} battle': 'Bataille {p0}',
    '{p0} card': 'Carte {p0}',
    '{p0}. {p1}': '{p0}. {p1}',
    'Attacking only': 'En attaque seulement',
    'Defending only': 'En défense seulement',
    Army: 'Armée', Fortress: 'Forteresse', Ship: 'Navire', City: 'Ville', or: 'ou',
    'Roll a die. Draw 1 card.': 'Lancez un dé. Piochez 1 carte.',
  } }, 'fr');
  for (const fighters of ['Army', 'Fortress', 'Ship', 'Army or Ship', 'Army or Fortress']) {
    const source = `Requires your ${fighters} in the battle`;
    const translated = `Nécessite votre ${fighters.split(' ').map((part) => translator.translate(part)).join(' ')} dans la bataille`;
    assert.equal(translator.translate(source), translated);
    assert.equal(translator.translate(source + '.'), translated + '.');
    assert.equal(
      translator.translate(`${source}. Roll a die. Draw 1 card.`),
      `${translated}. Lancez un dé. Piochez 1 carte.`,
    );
    assert.equal(
      translator.translate(` Attacking only · City battle · ${source}. Roll a die. Draw 1 card.\n`),
      ` En attaque seulement · Bataille Ville · ${translated}. Lancez un dé. Piochez 1 carte.\n`,
    );
  }
  assert.equal(
    translator.translate('Defending only · City battle. Roll a die. Draw 1 card.'),
    'En défense seulement · Bataille Ville. Lancez un dé. Piochez 1 carte.',
  );
  // Do not generalize this into splitting arbitrary prose at periods. Existing
  // authored-key coverage guards must still expose missing sentences intact.
  const unknown = 'An entirely new rule appears here. Army';
  assert.equal(translator.translate(unknown), unknown);
  translator.setNames(['Requires your Army in the battle.']);
  assert.equal(translator.translate('Requires your Army in the battle.'), 'Requires your Army in the battle.');
  translator.setLocale('en');
  assert.equal(translator.translate('Requires your Army in the battle.'), 'Requires your Army in the battle.');
});

test('ambiguous adjacent parameters are not guessed, and nested caches do not poison later translations', () => {
  const t = createTranslator({ fr: {
    'Build {p0}{p1}': 'incorrect {p0}{p1}',
    'Build {p0} at {p1}': 'Construire {p0} à {p1}',
    Temple: 'Temple',
    '{p0} army{p1}': '{p0} armée{p1}',
    Forest: 'Forêt',
    'city size {p0}, Fortress': 'ville de taille {p0}, Forteresse',
    '{p0} army ({p1} aboard)': '{p0} armée ({p1} à bord)',
  } }, 'fr');
  assert.equal(t.translate('Build Temple at C2'), 'Construire Temple à C2');
  assert.equal(t.translate('Forest · city size 3, Fortress · 2 army (1 aboard)'), 'Forêt · ville de taille 3, Forteresse · 2 armée (1 à bord)');
  assert.equal(t.translate('city size 3, Fortress'), 'ville de taille 3, Forteresse');
});

test('wonder research hints translate the complete required-for caption and private-hand subtitle', async () => {
  const { readFile } = await import('node:fs/promises');
  for (const locale of ['en', 'fr', 'de', 'da', 'el', 'fa', 'hi', 'it', 'ko', 'nl', 'pl', 'pt-BR', 'ro', 'ru', 'vi', 'zh-CN', 'zh-TW']) {
    const catalog = JSON.parse(await readFile(new URL(`./localization/${locale}.json`, import.meta.url), 'utf8'));
    const translator = createTranslator({ [locale]: catalog }, locale);
    for (const name of ['Great Library', 'Great Gardens']) {
      assert.equal(translator.translate(`Required for ${name}`), catalog['Required for {p0}'].replace('{p0}', catalog[name] ?? name), locale);
    }
    assert.equal(translator.translate('Wonder in your hand'), catalog['Wonder in your hand'], locale);
  }
});
