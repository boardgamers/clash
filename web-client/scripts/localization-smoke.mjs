// Run after `npm run build`: node scripts/localization-smoke.mjs
// This mounts the shipped viewer, not a test-only Svelte component. All HTTP
// requests are intercepted; no remote game, CDN, or development server is used.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const game = JSON.parse(await engine.init(2, [], { civilization: 'Random' }, 'localization-smoke', {}));
// Every printed objective card, including the five current exclusion clauses.
// This is a private-hand display fixture, not a legal deal or a gameplay test.
game.players[0].objective_cards = Array.from({ length: 38 }, (_, index) => index + 1);
const state = engine.stripSecret(JSON.stringify(game), 0);
const view = JSON.parse(engine.webView(state, 0));
const objectives = view.objectiveCards.flatMap((card) => card.objectives);
const restrictions = objectives.filter((objective) => objective.description.includes('Cannot be completed'));
assert.deepEqual(restrictions.map((objective) => objective.name).sort(),
  ['Colony', 'Military Might', 'Shipping Routes', 'Standing Army', 'Trade Power']);
const french = JSON.parse(await readFile(new URL('../dist/fr.json', import.meta.url), 'utf8'));
const normalize = (text) => text.replace(/\s+/g, ' ').trim();
for (const objective of restrictions) {
  const translation = french[normalize(objective.description)];
  assert(translation && translation !== objective.description,
    `Production French catalog must translate the complete current ${objective.name} description: ${objective.description}`);
  assert(!translation.includes('Cannot be completed'), `${objective.name}: untranslated restriction`);
}
const origin = 'https://boardgamers.space';
const assetPrefix = '/__clash_assets__/';
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});

async function selectLocale(page, locale) {
  await page.evaluate((locale) => host.emit('preferences', { sound: false, locale }), locale);
}
async function waitLocale(page, locale) {
  await page.waitForSelector(`[data-clash-locale="${locale}"]`);
}
async function waitText(page, selector, expected, attribute) {
  await page.waitForFunction(({ selector, expected, attribute }) => {
    const element = document.querySelector(selector);
    const actual = attribute ? element?.getAttribute(attribute) : element?.textContent;
    return actual?.replace(/\s+/g, ' ').trim() === expected;
  }, { selector, expected: normalize(expected), attribute });
}
async function assertFrenchObjectives(page) {
  for (const objective of restrictions) {
    const index = objectives.indexOf(objective);
    // Conditions are nested in cards; the flattened NodeList matches webView order.
    await page.waitForFunction(({ index, expected }) => {
      const paragraph = document.querySelectorAll('.objectives-dialog .objective-condition > p')[index];
      return paragraph?.textContent.replace(/\s+/g, ' ').trim() === expected;
    }, { index, expected: normalize(french[normalize(objective.description)]) });
  }
}

try {
  for (const width of [390, 1400]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [], consoleErrors = [], unexpectedRequests = [], failed = [];
    let frenchRequests = 0;
    const injectedFailures = new Set();
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') consoleErrors.push({ text: message.text(), url: message.location().url });
    });
    page.on('response', (response) => {
      if (response.status() >= 400 && !injectedFailures.has(response.url())) {
        failed.push(`${response.status()} ${response.url()}`);
      }
    });
    await page.route('**/*', async (route) => {
      const url = new URL(route.request().url());
      // The viewer's optional display-font stylesheet is unrelated to catalogs.
      // Stub it locally so this smoke remains deterministic and fully offline.
      if (url.origin === 'https://fonts.googleapis.com' && url.pathname === '/css2') {
        return route.fulfill({ contentType: 'text/css', body: '' });
      }
      if (url.origin === origin && url.pathname === '/__localization_check__') {
        return route.fulfill({ contentType: 'text/html', body:
          '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div>' });
      }
      if (url.origin !== origin || !url.pathname.startsWith(assetPrefix)) {
        unexpectedRequests.push(url.href);
        return route.abort();
      }
      const path = decodeURIComponent(url.pathname.slice(assetPrefix.length));
      assert(!path.includes('..') && !path.startsWith('/'));
      if (path === 'fr.json') frenchRequests++;
      if ((path === 'fr.json' && frenchRequests === 1) || path === 'de.json') {
        injectedFailures.add(url.href);
        return route.fulfill({ status: 503, contentType: 'text/plain', body: 'Deliberate localization smoke failure' });
      }
      const types = { json: 'application/json', wasm: 'application/wasm', woff2: 'font/woff2',
        js: 'text/javascript', css: 'text/css', svg: 'image/svg+xml', png: 'image/png', jpg: 'image/jpeg' };
      try {
        await route.fulfill({ contentType: types[path.split('.').pop()] || 'application/octet-stream',
          body: await readFile(new URL('../dist/' + path, import.meta.url)) });
      } catch (error) {
        failed.push(`${path}: ${error.message}`);
        await route.abort();
      }
    });
    await page.goto(`${origin}/__localization_check__`);
    await page.addScriptTag({ url: `${origin}${assetPrefix}viewer.js` });
    await page.evaluate((state) => {
      window.host = clash3d.launch('#app');
      window.readyCount = 0;
      host.on('ready', () => readyCount++);
      host.emit('preferences', { sound: false, locale: 'fr-CA' });
      host.emit('player', { index: 0 });
      host.emit('state', state);
    }, state);
    await page.waitForFunction(() => readyCount === 1);
    const banner = page.locator('[data-clash-locale-status]');
    await banner.waitFor({ state: 'visible' });
    await waitLocale(page, 'en');
    assert.equal(await banner.getByRole('status').getAttribute('aria-live'), 'polite');
    assert.match(await banner.getByRole('status').innerText(), /Could not load.*Showing English/);
    const retry = banner.getByRole('button', { name: /Retry language download.*Français/ });
    assert.equal(await retry.locator('bdi').getAttribute('lang'), 'fr');
    // No second preferences event: retry must retain the original fr-CA request.
    await retry.click();
    await waitLocale(page, 'fr');
    await banner.waitFor({ state: 'detached' });
    assert.equal(frenchRequests, 2);
    await waitText(page, '.objectives-button', 'Objectifs : 38 cartes', 'aria-label');
    await page.locator('.objectives-button').click();
    await page.locator('.objectives-dialog').waitFor({ state: 'visible' });
    assert.equal(await page.locator('.objectives-dialog .objective-condition').count(), objectives.length);
    await assertFrenchObjectives(page);
    if (width < 760) {
      const dialog = page.locator('.objectives-dialog');
      assert(await dialog.evaluate((element) => element.classList.contains('mobile-minimize-has-close')),
        'French close controls must reserve space for the positioned minimize button');
      const close = await dialog.locator('[data-panel-close]').boundingBox();
      const minimize = await dialog.locator('.mobile-panel-minimize').boundingBox();
      assert(close && minimize && (minimize.x + minimize.width <= close.x || close.x + close.width <= minimize.x),
        `Localized close and minimize controls must not overlap: ${JSON.stringify({ close, minimize })}`);
      await dialog.locator('.mobile-panel-minimize').click();
      assert.equal(await page.locator('dialog:modal').count(), 0);
      await page.locator('.restore-mobile-panel').click();
      await assertFrenchObjectives(page);
    }

    // Existing ResourceText nodes must retranslate, not retain old French or English.
    await selectLocale(page, 'en');
    await waitLocale(page, 'en');
    await waitText(page, '#objectives-title', 'Objectives');
    await selectLocale(page, 'fr-FR');
    await waitLocale(page, 'fr');
    await assertFrenchObjectives(page);
    assert.equal(await banner.count(), 0);
    assert.equal(frenchRequests, 2, 'Successful catalog should be cached across regional aliases');
    // Close must remain pointer-accessible beside the mobile minimize control.
    // Do not force-click: an overlap here is a real localized-layout regression.
    await page.locator('.objectives-dialog [data-panel-close]').click({ timeout: 5000 });
    if (width < 760) {
      // Also cover the header insertion branch, while the labels are French.
      await page.locator('[data-tutorial="research"]').click();
      const research = page.locator('dialog.research-dialog');
      await research.waitFor({ state: 'visible' });
      assert(await research.locator('header [data-panel-close]').evaluate((close) =>
        close.previousElementSibling?.classList.contains('mobile-panel-minimize')),
      'Minimize must be inserted immediately before the localized header close control');
      await research.locator('.mobile-panel-minimize').click();
      await page.locator('.restore-mobile-panel').click();
      await research.locator('header [data-panel-close]').click({ timeout: 5000 });
    }

    // Exercise a changing numeric caption, including singular/plural, on real UI.
    for (const actions of [1, 2]) {
      const changed = { ...game, actions_left: actions };
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(changed), 0));
      await waitText(page, '.action-markers', `Reste : ${actions} action${actions === 1 ? '' : 's'}`, 'aria-label');
    }

    // Selecting another language after failure must remove that stale retry UI.
    await selectLocale(page, 'de-DE');
    await banner.waitFor({ state: 'visible' });
    await waitLocale(page, 'en');
    await selectLocale(page, 'en');
    await banner.waitFor({ state: 'detached' });
    await selectLocale(page, 'fr-CA');
    await waitLocale(page, 'fr');
    await waitText(page, '.objectives-button', 'Objectifs : 38 cartes', 'aria-label');
    assert.equal(await banner.count(), 0);
    assert.equal(frenchRequests, 2);
    assert(await page.locator('canvas').count());
    const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth }));
    assert(geometry.scroll <= geometry.width + 1, JSON.stringify(geometry));
    assert.deepEqual(errors, []);
    assert.deepEqual(unexpectedRequests, []);
    assert.deepEqual(failed, []);
    // Chromium logs HTTP 503s at error level; permit only our injected requests.
    assert.deepEqual(consoleErrors.filter(({ text, url }) =>
      !(injectedFailures.has(url) && /503/.test(text))), []);
    console.log(`${width}px: five full French objective restrictions, dynamic captions, failed download/retry, regional aliases, switching and clean layout passed`);
    await page.close();
  }
} finally {
  await browser.close();
}
