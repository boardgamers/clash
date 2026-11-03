// Offline built-viewer regression: npm run build && node scripts/calendar-preview-smoke.mjs
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const view = (state, seat) => JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
const move = (state, action) => engine.tryMove(state, JSON.stringify(action), engine.currentPlayer(state));
let setup = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'calendar-browser', {});
setup = move(setup, { ChooseCivilization: 'Maya' });
setup = move(setup, { ChooseCivilization: 'Rome' });
const seat = engine.currentPlayer(setup);
const catalog = view(setup).eventCatalog;
function fixture(event = 'Great Merchant', culture = 2) {
  const game = JSON.parse(setup);
  const player = game.players[seat];
  player.advances = [...new Set([...player.advances, 'Fishing', 'Navigation', 'Cartography', 'Astronomy'])];
  player.resources.culture_tokens = culture;
  const id = catalog.find((card) => card.name === event).id;
  game.incidents_left = [id, ...game.incidents_left.filter((value) => value !== id)];
  const state = JSON.stringify(game);
  return move(state, view(state, seat).specialActions.find((action) => action.name === 'Calendar').action);
}

const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const locale of ['en', 'fr']) {
    const translations = locale === 'en' ? {} : JSON.parse(await readFile(new URL(`../dist/${locale}.json`, import.meta.url), 'utf8'));
    const t = (text) => translations[text] ?? text;
    for (const [width, height] of [[1400, 900], [390, 740]]) {
      const page = await browser.newPage({ viewport: { width, height } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname === 'fonts.googleapis.com') return route.fulfill({ contentType: 'text/css', body: '' });
        if (url.hostname !== 'clash.test') return route.abort();
        if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body:
          '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
        const path = decodeURIComponent(url.pathname.slice(1));
        assert.ok(!path.includes('..'));
        const types = { js: 'text/javascript', json: 'application/json', wasm: 'application/wasm', woff2: 'font/woff2', svg: 'image/svg+xml' };
        try {
          return route.fulfill({ contentType: types[path.split('.').at(-1)] ?? 'application/octet-stream',
            body: await readFile(new URL('../dist/' + path, import.meta.url)) });
        } catch { return route.abort(); }
      });
      await page.goto('http://clash.test/');
      await page.evaluate(({ seat, locale }) => {
        window.host = clash3d.launch('#app');
        window.sent = [];
        host.on('move', (action) => sent.push(action));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false, locale });
      }, { seat, locale });
      const emit = async (state, player = seat) => {
        await page.evaluate(({ state, player }) => {
          window.sent = [];
          host.emit('player', player === null ? {} : { index: player });
          host.emit('state', state);
        }, { state: engine.stripSecret(state, player ?? undefined), player });
      };
      const merchant = fixture();
      await emit(merchant);
      await page.waitForSelector(`[data-clash-locale="${locale}"]`);
      const preview = page.locator('.calendar-preview');
      await preview.getByRole('heading', { name: t('Great Merchant'), exact: true }).waitFor();
      assert.equal(await preview.getByRole('heading').count(), 4);
      for (const heading of ['Event effects', 'Card acquisition', 'Civil effect']) {
        await preview.getByRole('heading', { name: t(heading), exact: true }).waitFor();
      }
      const acquisition = preview.locator('.great-person-acquisition');
      const rule = 'The player who triggered the event may take the card for 1 culture token. If they pass, other players may take it in turn order for 2 culture tokens.';
      const acquisitionText = await acquisition.evaluate((node) => {
        const copy = node.cloneNode(true);
        for (const resource of copy.querySelectorAll('.inline-resource')) resource.textContent = resource.getAttribute('aria-label');
        return copy.textContent.replace(/\s+/g, ' ').trim();
      });
      assert.equal(acquisitionText, t(rule));
      assert.equal(await acquisition.locator('.inline-resource').count(), 2);
      const effect = preview.locator('.great-person-effect');
      assert.ok((await effect.innerText()).includes(t('Trade Routes')) || (await effect.innerText()).includes('routes commerciales'));
      assert.ok(!(await effect.innerText()).includes('If they pass'));
      const actions = page.locator('.calendar-actions');
      const bottom = actions.getByRole('button', { name: new RegExp(t('Put at the bottom')) });
      const keep = actions.getByRole('button', { name: t('Keep on top'), exact: true });
      assert.ok(await bottom.isEnabled());
      assert.ok(await keep.isEnabled());
      assert.equal(await page.locator('.decision-panel .decision-description').count(), 0, 'old wall of text is replaced, not duplicated');
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.locator('.decision-panel').evaluate((panel) => { panel.scrollTop = 0; });
      const panelBox = await page.locator('.decision-panel').boundingBox();
      const effectBox = await effect.boundingBox();
      assert.ok(effectBox.y >= panelBox.y && effectBox.y + effectBox.height <= panelBox.y + panelBox.height,
        'Great Person civil effect is readable before scrolling, including on mobile');
      if (width === 1400) {
        const controlsBox = await actions.boundingBox();
        assert.ok(controlsBox.y + controlsBox.height <= panelBox.y + panelBox.height,
          'desktop keep/bury controls fit inside the panel without scrolling');
      }
      await page.screenshot({ path: `/tmp/clash-calendar-${locale}-${width}.png` });
      await keep.scrollIntoViewIfNeeded();
      if (width === 390) await page.screenshot({ path: `/tmp/clash-calendar-${locale}-${width}-actions.png` });
      const clicked = width === 1400 ? bottom : keep;
      await clicked.click();
      await page.waitForFunction(() => sent.length === 1);
      const response = JSON.parse(await page.evaluate(() => sent[0]));
      assert.deepEqual(response, { Response: { Payment: [width === 1400 ? { culture_tokens: 1 } : {}] } });
      const result = JSON.parse(move(merchant, response));
      assert.equal(result.players[seat].resources.culture_tokens, width === 1400 ? 1 : 2);
      assert.equal(result.events?.length ?? 0, 0);
      await emit(JSON.stringify(result));
      await preview.waitFor({ state: 'detached' });
      await emit(merchant);
      await preview.waitFor();
      const cancel = page.locator('.decision-heading').getByRole('button', { name: t('Cancel ability'), exact: true });
      await cancel.click();
      await page.waitForFunction(() => sent.length === 1);
      assert.equal(JSON.parse(await page.evaluate(() => sent[0])), 'Undo');
      const closed = move(merchant, 'Undo');
      assert.equal(engine.isLiveUpdate(closed), true);
      assert.equal(engine.logLength(closed), engine.logLength(merchant));
      assert.deepEqual(engine.timeIncrements(closed), engine.timeIncrements(merchant));
      await emit(closed);
      await preview.waitFor({ state: 'detached' });
      await emit(fixture('Great Merchant', 0));
      await preview.waitFor();
      assert.equal(await actions.getByRole('button').count(), 1);
      assert.ok(await actions.getByRole('button', { name: t('Keep on top'), exact: true }).isEnabled());
      await emit(fixture('Great Diplomat'));
      await preview.getByRole('heading', { name: t('Great Diplomat'), exact: true }).waitFor();
      assert.ok((await effect.innerText()).includes(t('Active when taken')));
      await emit(fixture('Famine'));
      await page.locator('.decision-panel .payment-choices').waitFor();
      assert.equal(await preview.count(), 0, 'ordinary event UI remains unchanged');
      assert.ok(await cancel.isEnabled(), 'ordinary Calendar previews can also be cancelled');
      await emit(merchant, 1 - seat);
      await preview.waitFor({ state: 'detached' });
      assert.equal(await page.locator('.decision-panel').count(), 0, 'opponent cannot see or act on this preview');

      // The real Undo control must navigate a Calendar-known incident, not just
      // close a look-only prompt. It must not offer a second undo past the peek.
      const data = JSON.parse(setup);
      const owner = data.players[seat];
      owner.advances = [...new Set([...owner.advances, 'Math', 'Astronomy'])];
      owner.resources.food = 7;
      owner.incident_tokens = 1;
      data.incidents_left = [29, 30]; // Vulcan: no effect with one city.
      for (const player of data.players) player.objective_cards = [];
      const wedge = view(setup).cardCatalog.find(card => card.tactics?.name === 'Wedge Formation');
      owner.action_cards = [wedge.id];
      const beforeIncident = JSON.stringify(data);
      await emit(beforeIncident);
      const triggered = move(beforeIncident, { Playing: { Advance: { advance: 'Storage', payment: { food: 2 } } } });
      await emit(triggered);
      await page.getByRole('button', { name: t('Dismiss event explanation'), exact: true }).click();
      const undo = page.locator('[data-tutorial="undo"]');
      await undo.waitFor();
      assert.ok(await undo.isEnabled());
      await undo.click();
      await page.waitForFunction(() => sent.length === 1);
      assert.equal(JSON.parse(await page.evaluate(() => sent[0])), 'Undo');
      const undoneIncident = move(triggered, 'Undo');
      assert.equal(view(undoneIncident, seat).canUndo, false);
      assert.deepEqual(engine.timeIncrements(undoneIncident), engine.timeIncrements(beforeIncident));
      await emit(undoneIncident);
      await undo.click();
      await page.locator('.undo-options button').waitFor();
      assert.deepEqual(await page.evaluate(() => sent), [], 'the second click opens Redo, not another Undo');
      await page.locator('.undo-options button').click();
      await page.waitForFunction(() => sent.length === 1);
      assert.equal(JSON.parse(await page.evaluate(() => sent[0])), 'Redo');
      await emit(move(undoneIncident, 'Redo'));

      // This is the same ResourceText path that displayed mixed French/English.
      await page.locator('[data-tutorial="cards"]').click();
      const battleRule = page.locator('.cards-dialog .card-battle-use p');
      await battleRule.waitFor();
      assert.equal((await battleRule.innerText()).trim(), locale === 'fr'
        ? "En attaque seulement · Nécessite votre Armée dans la bataille. Recevez 1 en valeur de combat pour chaque unité d'armée en défense"
        : wedge.tactics.description);
      await page.screenshot({ path: `/tmp/clash-wedge-${locale}-${width}.png` });
      assert.deepEqual(errors, []);
      await page.close();
      console.log(`${locale} ${width}: Calendar preview/privacy, incident Undo/Redo and complete tactics translation passed.`);
    }
  }
} finally { await browser.close(); }
