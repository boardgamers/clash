// Offline built-viewer regression: npm run build && node scripts/research-reminders-smoke.mjs
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'research-reminders-browser', {})).players.slice(2);
async function fixture(name = 'action_cards/synergies') {
  const g = JSON.parse(await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'));
  for (const npc of npcs) if (!g.players.some((p) => p.civilization === npc.civilization)) g.players.push({ ...npc, id: g.players.length });
  g.players[0].advances = ['Farming', 'Mining', 'Math', 'Tactics', 'Bartering'];
  g.players[0].action_cards = [122];
  g.players[0].incident_tokens = 3;
  g.players[1].action_cards = [];
  return g;
}
const v = (g) => JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(g), 0), 0));
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const locale of ['en', 'fr']) {
    const translations = locale === 'en' ? {} : JSON.parse(await readFile(new URL('../dist/fr.json', import.meta.url), 'utf8'));
    const t = (text) => translations[text] ?? text;
    for (const [width, height] of [[1400, 900], [390, 740]]) {
      const page = await browser.newPage({ viewport: { width, height } });
      page.setDefaultTimeout(30000);
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname === 'fonts.googleapis.com') return route.fulfill({ contentType: 'text/css', body: '' });
        if (url.hostname !== 'clash.test') return route.abort();
        if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body:
          '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
        const path = decodeURIComponent(url.pathname.slice(1));
        assert(!path.includes('..'));
        const types = { js: 'text/javascript', json: 'application/json', wasm: 'application/wasm', woff2: 'font/woff2', svg: 'image/svg+xml' };
        try { return route.fulfill({ contentType: types[path.split('.').at(-1)] ?? 'application/octet-stream', body: await readFile(new URL('../dist/' + path, import.meta.url)) }); }
        catch { return route.abort(); }
      });
      async function mount(game, seat = 0) {
        await page.goto('http://clash.test/');
        await page.evaluate(({ state, seat, locale }) => {
          window.host = clash3d.launch('#app'); window.sent = [];
          host.on('move', (action) => sent.push(action));
          host.emit('player', seat === null ? {} : { index: seat });
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false, locale });
          host.emit('state', state);
        }, { state: engine.stripSecret(JSON.stringify(game), seat ?? undefined), seat, locale });
        await page.waitForSelector(`[data-clash-locale="${locale}"]`);
      }
      const open = async (id) => {
        await page.getByRole('button', { name: t('Research tree'), exact: true }).click();
        await page.locator(`#research-${id} .research-pick`).click();
      };
      const sent = () => page.evaluate(() => window.sent.map((a) => JSON.parse(a)));
      const detail = page.locator('.research-detail');
      const alternatives = detail.locator('.research-alternatives');
      const primary = detail.locator('.research-payment > .primary');

      // Merely opening/selecting an advance never spends either resources or a card.
      let game = await fixture();
      await mount(game); await open('Medicine');
      const provinceLink = page.locator('.research-civilization-link').filter({ hasText: t('Provinces') }).first();
      assert.equal(await provinceLink.getByText(t('Unlocks'), { exact: true }).count(), 1);
      await alternatives.getByText(t('Alternative research available'), { exact: true }).waitFor();
      assert((await alternatives.innerText()).includes(t('No research resource cost')));
      assert.deepEqual(await sent(), []);
      assert(await primary.isEnabled(), 'normal paid research remains an explicit separate option');
      await primary.scrollIntoViewIfNeeded();
      await page.screenshot({ path: `/tmp/clash-research-reminder-${locale}-${width}-paid-choice.png` });
      await primary.click();
      await page.waitForFunction(() => sent.length === 1);
      assert.deepEqual((await sent())[0], v(game).advances.find((a) => a.id === 'Medicine').action);
      const paid = JSON.parse(engine.tryMove(JSON.stringify(game), JSON.stringify((await sent())[0]), 0));
      assert(paid.players[0].action_cards.includes(122), 'paid research does not silently consume Scientist');

      for (const [id, name, target] of [[122, 'Great Scientist', 'Medicine'], [125, 'Great Merchant', 'TradeRoutes'], [124, 'Great Warlord', 'SteelWeapons']]) {
        game = await fixture(); game.players[0].action_cards = [id];
        await mount(game); await open(target);
        assert.deepEqual(await sent(), []);
        const use = alternatives.locator('.research-source');
        assert.equal(await use.count(), 1);
        assert((await use.innerText()).includes(t(name)));
        assert((await page.locator(`#research-${target} .research-card-hint`).innerText()).includes(t(name)));
        await use.scrollIntoViewIfNeeded();
        assert(await use.isVisible());
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
        await page.screenshot({ path: `/tmp/clash-research-reminder-${locale}-${width}-${id}.png` });
        await use.click();
        await page.waitForFunction(() => sent.length === 1);
        assert.deepEqual((await sent())[0], { Playing: { ActionCard: id } });
        const requested = engine.tryMove(JSON.stringify(game), JSON.stringify((await sent())[0]), 0);
        await page.evaluate((state) => { window.sent = []; host.emit('state', state); }, engine.stripSecret(requested, 0));
        await page.locator('.research-dialog').waitFor();
        assert.equal(await page.locator('.research-alternatives').count(), 0, 'no redundant reminders inside an existing free-advance choice');
        assert.equal(await page.locator('.research-card-hint').count(), 0);
        await page.locator(`#research-${target} .research-pick`).click();
        assert.deepEqual(await sent(), []);
        await primary.click();
        await page.waitForFunction(() => sent.length === 1);
        assert.deepEqual((await sent())[0], { Response: { SelectAdvance: target } });
      }

      // Leader alternatives use the real custom action rather than paid Advance.
      game = await fixture('civilizations/greece/master');
      game.players[0].action_cards = [];
      game.players[0].advances.push('Writing');
      await mount(game); await open('Philosophy');
      assert((await alternatives.innerText()).includes(t('Master')));
      assert.deepEqual(await sent(), []);
      await alternatives.locator('.research-source').click();
      await page.waitForFunction(() => sent.length === 1);
      assert.deepEqual((await sent())[0], { Playing: { Custom: { action: 'Master' } } });

      // Keep Synergies' specific two-advance benefit alongside the new reminders.
      game = await fixture(); game.players[0].action_cards = [33];
      await mount(game); await open('Medicine');
      assert((await alternatives.innerText()).includes(t('· 2 advances in this category')), await alternatives.innerText());
      assert((await alternatives.innerText()).includes(t('Pay research cost next')));
      const groupHint = page.locator('.research-branch > .research-card-hint').first();
      assert((await groupHint.innerText()).includes(t('· 2 advances · normal resource costs')), await groupHint.innerText());
      assert.deepEqual(await sent(), []);
      await alternatives.locator('.research-source').click();
      await page.waitForFunction(() => sent.length === 1);
      assert.deepEqual((await sent())[0], { Playing: { ActionCard: 33 } });

      // A private opponent's hand and an exhausted action allowance never make reminders.
      game = await fixture();
      game.players[0].action_cards = [];
      game.players[1].action_cards = [122, 125, 124];
      await mount(game); await open('Medicine');
      assert.equal(await page.locator('.research-alternatives, .research-card-hint').count(), 0);
      game.players[0].action_cards = [122]; game.actions_left = 0;
      await mount(game);
      await page.getByRole('button', { name: t('Research tree'), exact: true }).click();
      assert.equal(await page.locator('.research-card-hint').count(), 0);
      assert.deepEqual(errors, []);
      await page.close();
      console.log(`${locale} ${width}: deliberate paid research, Scientist/Merchant/Warlord and leader shortcuts, free-choice suppression, privacy and action limits passed.`);
    }
  }
} finally { await browser.close(); }
