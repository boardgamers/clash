// Run after `npm run build`: node scripts/great-person-localization-smoke.mjs
// Mount the shipped viewer with a real Great Scientist event, both as the
// deciding player and as an off-turn player with an early card offer.
// All requests (including optional display fonts) are intercepted locally.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const normalize = (text) => text.replace(/\s+/g, ' ').trim();
const acquisition = 'The player who triggered the event may take the card for 1 culture token. If they pass, other players may take it in turn order for 2 culture tokens.';
const french = JSON.parse(await readFile(new URL('../dist/fr.json', import.meta.url), 'utf8'));
const translated = french[acquisition];
assert(translated && translated !== acquisition,
  `Production French catalog must translate the complete acquisition rule: ${acquisition}`);
assert.match(translated, /Le joueur qui a déclenché/);
assert.match(translated, /1 jeton de culture/);
assert.match(translated, /2 jetons de culture/);
assert.doesNotMatch(translated, /The player|triggered the event|If they pass|other players|turn order|culture tokens?/i);

const engine = createRequire(import.meta.url)('../.engine/server.js');
const game = JSON.parse(await readFile(
  new URL('../../server/tests/test_games/incidents/great_persons/great_artist.json', import.meta.url), 'utf8',
));
// Reuse the small, legal Great Person setup. Great Scientist is event 22;
// both players can afford the offer so both production UI branches exist.
const fresh = JSON.parse(await engine.init(2, [], {}, 'great-person-localization-smoke', {}));
for (const npc of fresh.players.slice(2)) {
  if (!game.players.some((player) => player.civilization === npc.civilization)) {
    game.players.push({ ...npc, id: game.players.length });
  }
}
game.incidents_left = [22];
game.players[0].resources.culture_tokens = 3;
const eventState = engine.tryMove(JSON.stringify(game), JSON.stringify({
  Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } },
}), 0);
const fixtures = [0, 1].map((seat) => {
  const state = engine.stripSecret(eventState, seat);
  return { seat, state, view: JSON.parse(engine.webView(state, seat)) };
});
const decision = fixtures[0].view.decision;
assert.equal(decision?.name, 'Great Scientist');
assert.equal(decision.eventContext?.card?.name, 'Great Scientist');
assert.equal(decision.eventContext.card.later, false);
assert.deepEqual(decision.eventContext.card.cost, { culture_tokens: 1 });
assert(decision.fields.some((field) => field.name === 'Pay to gain the Action Card'));
assert.equal(fixtures[1].view.activePlayer, 0);
assert.equal(fixtures[1].view.decision, null);
const offers = fixtures[1].view.earlyChoices.filter((choice) => choice.key.endsWith('/offer'));
assert.equal(offers.length, 1);
assert.equal(offers[0].title, 'Great Scientist');
assert(offers[0].options.some((option) => option.response.Payment[0].culture_tokens === 2));

const origin = 'https://boardgamers.space';
const assetPrefix = '/__clash_assets__/';
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});

async function assertRule(page, selector, expected, locale) {
  const rule = page.locator(selector);
  assert.equal(await rule.count(), 1, 'Exactly one shared acquisition paragraph should be shown');
  await rule.waitFor({ state: 'visible' });
  // Compact icons render only the number. Restore their accessible resource
  // labels before comparing the complete translated sentence.
  await page.waitForFunction(({ selector, expected }) => {
    const copy = document.querySelector(selector)?.cloneNode(true);
    if (!copy) return false;
    for (const resource of copy.querySelectorAll('.inline-resource')) {
      resource.textContent = resource.getAttribute('aria-label');
    }
    return copy.textContent.replace(/\s+/g, ' ').trim() === expected;
  }, { selector, expected: normalize(expected) });
  if (locale === 'fr') {
    assert.doesNotMatch(await rule.innerText(),
      /The player|triggered the event|If they pass|other players|turn order|culture tokens?/i,
      'Neither half of the acquisition rule may fall back to English');
  }
  const resources = rule.locator('.inline-resource');
  assert.equal(await resources.count(), 2, 'Both culture costs must retain their inline resource rendering');
  assert.deepEqual((await resources.allTextContents()).map((text) => normalize(text).match(/^\d+/)?.[0]), ['1', '2']);
  assert.equal(await resources.locator('svg.lucide-drama[aria-hidden="true"]').count(), 2,
    'Both costs must retain their culture icons');
}

try {
  for (const [width, height] of [[1400, 900], [390, 740]]) {
    for (const { seat, state } of fixtures) {
      const page = await browser.newPage({ viewport: { width, height } });
      page.setDefaultTimeout(15000);
      const errors = [], consoleErrors = [], unexpectedRequests = [], failed = [];
      page.on('pageerror', (error) => errors.push(error.message));
      page.on('console', (message) => {
        if (message.type() === 'error') consoleErrors.push(message.text());
      });
      page.on('response', (response) => {
        if (response.status() >= 400) failed.push(`${response.status()} ${response.url()}`);
      });
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        // Same offline display-font stub as localization-smoke.mjs.
        if (url.origin === 'https://fonts.googleapis.com' && url.pathname === '/css2') {
          return route.fulfill({ contentType: 'text/css', body: '' });
        }
        if (url.origin === origin && url.pathname === '/__great_person_localization_check__') {
          return route.fulfill({ contentType: 'text/html', body:
            '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div>' });
        }
        if (url.origin !== origin || !url.pathname.startsWith(assetPrefix)) {
          unexpectedRequests.push(url.href);
          return route.abort();
        }
        const path = decodeURIComponent(url.pathname.slice(assetPrefix.length));
        assert(!path.includes('..') && !path.startsWith('/'));
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
      await page.goto(`${origin}/__great_person_localization_check__`);
      await page.addScriptTag({ url: `${origin}${assetPrefix}viewer.js` });
      await page.evaluate(({ seat, state }) => {
        window.host = clash3d.launch('#app');
        host.emit('preferences', { sound: false, locale: 'fr-CA' });
        host.emit('player', { index: seat });
        host.emit('state', state);
      }, { seat, state });
      await page.waitForSelector('[data-clash-locale="fr"]');
      // The real event log can show its explanation overlay on first mount.
      // Dismiss it through the UI so it does not cover the mobile disclosure.
      const dismissEvent = page.locator('.card-reveal .reveal-title .icon-button');
      while (await dismissEvent.count()) await dismissEvent.first().click();

      const container = seat === 0 ? '.decision-panel' : '.cards-dialog .play-card[aria-label]';
      if (seat === 1) {
        await page.locator('.cards-button').click();
        await page.locator('.cards-dialog').waitFor({ state: 'visible' });
      }
      // Open the real Event rules disclosure, rather than inspecting hidden text.
      await page.locator(`${container} .event-rules > summary`).click();
      const selector = `${container} .great-person-acquisition`;
      await assertRule(page, selector, translated, 'fr');
      assert.equal(await page.locator('[data-clash-locale-status]').count(), 0);

      // The mounted ResourceText must also retranslate when preferences change.
      for (const [locale, resolved, expected] of [['en', 'en', acquisition], ['fr-FR', 'fr', translated]]) {
        await page.evaluate((locale) => host.emit('preferences', { sound: false, locale }), locale);
        await page.waitForSelector(`[data-clash-locale="${resolved}"]`);
        await assertRule(page, selector, expected, resolved);
      }
      assert(await page.locator('canvas').count());
      const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth }));
      assert(geometry.scroll <= geometry.width + 1, JSON.stringify(geometry));
      assert.deepEqual(errors, []);
      assert.deepEqual(consoleErrors, []);
      assert.deepEqual(unexpectedRequests, []);
      assert.deepEqual(failed, []);
      console.log(`${width}px ${seat === 0 ? 'decision' : 'off-turn card offer'}: complete French Great Scientist acquisition, culture costs 1/2, locale switching and clean layout passed`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
