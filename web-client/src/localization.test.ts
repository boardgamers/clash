import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTranslator, resolveLocale } from './localization/runtime.js';

test('resolves regional BGS language preferences', () => {
  assert.equal(resolveLocale('pt-PT'), 'pt-BR');
  assert.equal(resolveLocale('zh-Hant-HK'), 'zh-TW');
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
