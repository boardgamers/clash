// Offline built-viewer regression: npm run build && node scripts/spy-target-counts-smoke.mjs
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const game = JSON.parse(await engine.init(4, [], {}, 'spy-target-counts', {}));
game.current_player_index = 0;
const hands = [
  { action_cards: [7, 1], objective_cards: [1], wonder_cards: ['GreatWall'] },
  // Architect (155) / Diplomat (157) are public; Scientist (122) still counts.
  { action_cards: [2, 3, 122, 155], objective_cards: [2, 3], wonder_cards: [] },
  { action_cards: [157], objective_cards: [], wonder_cards: ['Pyramids'] },
  { action_cards: [5, 6], objective_cards: [4], wonder_cards: ['GreatGardens', 'GreatLibrary'] },
];
for (let i = 0; i < 4; i++) {
  Object.assign(game.players[i], hands[i]);
  game.players[i].civilization = ['Rome', 'Greece', 'Egypt', 'China'][i];
}
game.players[0].resources.culture_tokens = 3;
const move = (state, action) => engine.tryMove(state, JSON.stringify(action), 0);
const targets = move(move(JSON.stringify(game), { Playing: { ActionCard: 7 } }), {
  Response: { Payment: [{ culture_tokens: 1 }] },
});
const expected = [
  { player: 1, name: 'Greece', counts: { action: 3, objective: 2, wonder: 0 } },
  { player: 2, name: 'Egypt', counts: { action: 0, objective: 0, wonder: 1 } },
  { player: 3, name: 'China', counts: { action: 2, objective: 1, wonder: 2 } },
];
const labels = { action: 'Action', objective: 'Objectives', wonder: 'Wonders' };
const normalize = (text) => text.replace(/\s+/g, ' ').trim();
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const locale of ['en', 'fr']) {
    const translations =
      locale === 'en' ? {} : JSON.parse(await readFile(new URL('../dist/fr.json', import.meta.url), 'utf8'));
    const t = (text) => translations[text] ?? text;
    for (const [width, height] of [
      [1400, 900],
      [390, 844],
      [320, 640],
    ]) {
      const page = await browser.newPage({ viewport: { width, height } });
      page.setDefaultTimeout(30000);
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname === 'fonts.googleapis.com')
          return route.fulfill({ contentType: 'text/css', body: '' });
        if (url.hostname !== 'clash.test') return route.abort();
        if (url.pathname === '/')
          return route.fulfill({
            contentType: 'text/html',
            body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
          });
        const path = decodeURIComponent(url.pathname.slice(1));
        assert(!path.includes('..'));
        const types = {
          js: 'text/javascript',
          json: 'application/json',
          wasm: 'application/wasm',
          woff2: 'font/woff2',
          svg: 'image/svg+xml',
        };
        try {
          return route.fulfill({
            contentType: types[path.split('.').at(-1)] ?? 'application/octet-stream',
            body: await readFile(new URL('../dist/' + path, import.meta.url)),
          });
        } catch {
          return route.abort();
        }
      });
      await page.goto('http://clash.test/');
      await page.evaluate(
        ({ state, locale }) => {
          window.host = clash3d.launch('#app');
          window.sent = [];
          host.on('move', (action) => sent.push(JSON.parse(action)));
          host.emit('player', { index: 0 });
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false, locale });
          host.emit('state', state);
        },
        { state: engine.stripSecret(targets, 0), locale },
      );
      await page.waitForSelector(`[data-clash-locale="${locale}"] .decision-hand-counts`);
      const panel = page.locator('.decision-panel');
      const buttons = panel.locator('.decision-option');
      const confirm = panel.getByRole('button', { name: t('Confirm'), exact: true });
      const sent = () => page.evaluate(() => window.sent);
      assert.equal(await buttons.count(), expected.length);
      assert.equal(await panel.locator('.decision-hand-counts').count(), expected.length);
      assert.equal(
        await panel.locator('.decision-card-face, .decision-card-owner, .decision-rule').count(),
        0,
        'target choices must not reveal card identities or rules',
      );
      assert.equal(await confirm.isEnabled(), false);
      assert.deepEqual(await sent(), []);

      // Inspect Chromium's computed description, not merely referenced DOM text:
      // a nested group's aria-label can hide all of its badge text from that description.
      const accessibility = await page.context().newCDPSession(page);
      const { nodes } = await accessibility.send('Accessibility.getFullAXTree');
      await accessibility.detach();
      for (const target of expected) {
        const button = panel.getByRole('button', { name: t(target.name), exact: true });
        assert.equal(await button.getAttribute('aria-pressed'), 'false');
        assert.equal(await button.locator('.decision-hand-counts').count(), 1);
        assert.equal(await button.locator('.decision-hand-count').count(), 3);
        assert.ok(await button.getAttribute('aria-describedby'));
        const accessible = nodes.find(
          (node) => node.role?.value === 'button' && node.name?.value === t(target.name),
        );
        assert.ok(accessible, `${target.name} is a labelled accessible button`);
        const description = normalize(accessible.description?.value ?? '');
        assert.ok(
          description.includes(t(target.name)),
          'the description references this target, not another option',
        );
        for (const [kind, count] of Object.entries(target.counts)) {
          const badge = button.locator(`.decision-hand-count[data-card-kind="${kind}"]`);
          assert.equal(await badge.locator('b').innerText(), String(count), `${target.name} ${kind}`);
          assert.equal(await badge.locator('svg[aria-hidden="true"]').count(), 1);
          assert.ok(
            (await badge.innerText()).includes(t(labels[kind])),
            `${locale}: translated ${kind} label`,
          );
          assert.ok(
            description.includes(normalize(await badge.textContent())),
            `${target.name} ${kind} count must be in the computed accessible description; got ${JSON.stringify(description)}`,
          );
          assert.ok(
            await badge.evaluate((element) => {
              const rect = element.getBoundingClientRect();
              const parent = element.closest('button').getBoundingClientRect();
              return (
                rect.left >= Math.max(0, parent.left) - 1 &&
                rect.right <= Math.min(innerWidth, parent.right) + 1
              );
            }),
            `${locale} ${width}: ${target.name} ${kind} badge fits inside its button and viewport`,
          );
        }
        assert.ok(await button.evaluate((element) => element.scrollWidth <= element.clientWidth + 1));
      }
      assert.ok(await panel.evaluate((element) => element.scrollWidth <= element.clientWidth + 1));
      assert.ok(
        await panel
          .locator('.decision-options')
          .evaluate((element) => element.scrollWidth <= element.clientWidth + 1),
      );
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));

      const first = panel.getByRole('button', { name: t(expected[0].name), exact: true });
      const last = panel.getByRole('button', { name: t(expected.at(-1).name), exact: true });
      await first.click();
      assert.equal(await first.getAttribute('aria-pressed'), 'true');
      assert.equal(await confirm.isEnabled(), true);
      assert.deepEqual(await sent(), [], 'choosing a target must not send a move');
      await last.click();
      assert.equal(await first.getAttribute('aria-pressed'), 'false');
      assert.equal(await last.getAttribute('aria-pressed'), 'true');
      assert.equal(await panel.locator('.decision-option[aria-pressed="true"]').count(), 1);
      await panel.evaluate((element) => {
        element.scrollTop = 0;
      });
      await page.screenshot({ path: `/tmp/clash-spy-targets-${locale}-${width}.png` });
      assert.deepEqual(await sent(), [], 'changing the target still requires explicit confirmation');
      await confirm.click();
      await page.waitForFunction(() => sent.length === 1);
      const action = { Response: { SelectPlayer: expected.at(-1).player } };
      assert.deepEqual(await sent(), [action], 'Confirm submits the player id, not the option index');

      const swap = move(targets, action);
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(swap, 0));
      await page.waitForFunction(
        () =>
          document.querySelector('.decision-card-face') && !document.querySelector('.decision-hand-counts'),
      );
      assert.equal(
        await page.locator('.decision-hand-counts, .decision-hand-count').count(),
        0,
        'hand counts disappear at the card-swap stage',
      );
      assert.ok((await panel.locator('.decision-card-owner').count()) > 0);
      assert.deepEqual(await sent(), [action], 'showing the hand does not submit a swap');
      assert.deepEqual(errors, []);
      await page.close();
      console.log(
        `${locale} ${width}: localized counts (including zero), privacy, accessible descriptions, fit, explicit target confirmation and swap-stage cleanup passed.`,
      );
    }
  }
} finally {
  await browser.close();
}
