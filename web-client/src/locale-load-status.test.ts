import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createLocaleSelection } from './localization/load-status.js';
import { createCatalogLoader } from './localization/loader.js';

// Minimal DOM fixture: exercise the public retry handler and cleanup without Vite assets.
function fixture() {
  const document: any = {
    createElement: () => element(),
    createTextNode: (textContent: string) => ({ textContent }),
  };
  function element(): any {
    return {
      ownerDocument: document,
      children: [],
      attributes: {},
      style: {},
      onclick: null,
      setAttribute(key: string, value: string) {
        this.attributes[key] = value;
      },
      append(...nodes: any[]) {
        for (const node of nodes) {
          node.parent = this;
          this.children.push(node);
        }
      },
      remove() {
        if (this.parent) this.parent.children = this.parent.children.filter((node: any) => node !== this);
      },
    };
  }
  return element();
}

test('failed download exposes an accessible retry and retry downloads the requested locale', async () => {
  const target = fixture();
  const applied: string[] = [];
  let attempts = 0;
  const loader = createCatalogLoader({ './fr.json': '/fr.json' }, async () => {
    attempts++;
    return attempts === 1
      ? { ok: false, status: 503 }
      : { ok: true, json: async () => ({ Research: 'Recherche' }) };
  });
  const selection = createLocaleSelection(target, loader.load, (locale: string) => applied.push(locale));
  await selection.setLocale('fr-CA');
  assert.deepEqual(applied, ['en']);
  const banner = target.children[0];
  assert.equal(banner.attributes.translate, 'no');
  assert.equal(banner.children[0].attributes.role, 'status');
  assert.equal(banner.children[0].textContent, 'Could not load the selected language. Showing English.');
  const button = banner.children[1];
  assert.equal(button.children[0].textContent, 'Retry language download');
  assert.equal(button.children[2].textContent, 'Français');
  button.onclick();
  await selection.ready;
  assert.equal(attempts, 2);
  assert.deepEqual(applied, ['en', 'fr']);
  assert.equal(target.children.length, 0);
  assert.equal(button.onclick, null);
  selection.destroy();
});

test('stale rejected download cannot replace a newer locale or show a banner', async () => {
  const target = fixture();
  let reject!: (error: Error) => void;
  const pending = new Promise<string>((_, fail) => {
    reject = fail;
  });
  const applied: string[] = [];
  const selection = createLocaleSelection(
    target,
    (locale: string) => (locale === 'fr' ? pending : Promise.resolve(locale)),
    (locale: string) => applied.push(locale),
  );
  const old = selection.setLocale('fr');
  await selection.setLocale('de');
  reject(new Error('offline'));
  assert.equal(await old, false);
  assert.deepEqual(applied, ['de']);
  assert.equal(target.children.length, 0);
  selection.destroy();
});

test('destroy removes banner and retry handler and ignores in-flight failures', async () => {
  const target = fixture();
  const selection = createLocaleSelection(
    target,
    () => Promise.reject(new Error('offline')),
    () => {},
  );
  await selection.setLocale('fr');
  const button = target.children[0].children[1];
  selection.destroy();
  assert.equal(target.children.length, 0);
  assert.equal(button.onclick, null);
  assert.equal(await selection.setLocale('fr'), false);

  let reject!: (error: Error) => void;
  const pending = new Promise<string>((_, fail) => {
    reject = fail;
  });
  const inFlight = createLocaleSelection(
    target,
    () => pending,
    () => assert.fail('must not apply'),
  );
  const result = inFlight.setLocale('fr');
  inFlight.destroy();
  reject(new Error('offline'));
  assert.equal(await result, false);
  assert.equal(target.children.length, 0);
});
