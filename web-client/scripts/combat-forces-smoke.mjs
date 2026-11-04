// Offline regression against the fresh production viewer; no BGS moves are sent.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
function battle() {
  const g = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/combat/direct_capture_city_fortress.json', import.meta.url),
      'utf8',
    ),
  );
  g.players[0].advances = ['Farming', 'Mining', 'Tactics', 'SteelWeapons'];
  g.players[1].advances = ['Farming', 'Mining', 'Tactics'];
  g.players[0].units = [0, 1, 2, 3].map((id) => ({ id, position: 'C2', unit_type: 'Infantry' }));
  g.players[1].units = [0, 1].map((id) => ({ id, position: 'C1', unit_type: 'Infantry' }));
  g.players[1].cities = [{ position: 'C1', mood_state: 'Happy', city_pieces: { fortress: 1 } }];
  g.events = [
    {
      event_type: {
        CombatRoundStart: {
          phase: 'Default',
          combat: {
            attackers: [0, 1, 2],
            retreat: 'CanRetreat',
            modifiers: [],
            stats: {
              round: 1,
              battleground: 'CityWithFortress',
              attacker: { player: 0, position: 'C2', present: { infantry: 3 } },
              defender: { player: 1, position: 'C1', present: { infantry: 2 } },
            },
          },
          attacker_strength: {},
          defender_strength: {},
        },
      },
      players_used: [0],
      player: 1,
      last_priority_used: 0,
      handler: {
        priority: 0,
        request: {
          SelectHandCards: { choices: [], needed: { start: 0, end: 1 }, description: 'Play Tactics Card' },
        },
        origin: { Advance: 'Tactics' },
      },
    },
  ];
  return g;
}
const npcs = JSON.parse(await engine.init(2, [], {}, 'combat-forces-smoke', {})).players.slice(2);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const locale of ['en', 'fr'])
    for (const [width, height] of [
      [1400, 900],
      [390, 740],
    ]) {
      const page = await browser.newPage({ viewport: { width, height } });
      page.setDefaultTimeout(30000);
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route('**/*', async (route) => {
        const url = new URL(route.request().url());
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
      const game = battle();
      for (const npc of npcs)
        if (!game.players.some((p) => p.civilization === npc.civilization))
          game.players.push({ ...npc, id: game.players.length });
      await page.goto('http://clash.test/');
      await page.evaluate(
        ({ state, locale }) => {
          window.host = clash3d.launch('#app');
          window.sent = [];
          window.hovered = [];
          host.on('move', (action) => sent.push(action));
          host.on('player:hovered', (data) => hovered.push(data));
          host.emit('player', { index: 1 });
          host.emit('avatars', ['data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg"/>']);
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false, locale });
          host.emit('state', state);
        },
        { state: engine.stripSecret(JSON.stringify(game), 1), locale },
      );
      const table = page.locator('.combat-forces');
      await table.waitFor();
      const rows = table.locator('tbody tr');
      assert.deepEqual(await rows.nth(0).locator('td b').allTextContents(), ['3', '2']);
      assert.deepEqual(await rows.nth(1).locator('td b').allTextContents(), ['3', '3']);
      assert.match(await rows.nth(3).innerText(), locale === 'en' ? /not activated/ : /non activées/);
      assert.equal(await page.getByText('Play one battle effect or skip.', { exact: true }).count(), 0);
      game.events[0].event_type.CombatRoundStart.combat.modifiers.push('SteelWeaponsAttacker');
      await page.evaluate((state) => host.emit('state', state), engine.stripSecret(JSON.stringify(game), 1));
      await page.waitForFunction(
        () => document.querySelector('.combat-forces tbody tr:nth-child(3) td')?.textContent?.trim() === '+2',
      );
      assert.deepEqual(await rows.nth(1).locator('td b').allTextContents(), ['3', '3']);
      const card = page.locator('.player-card').first();
      if (width >= 1100) {
        await card.locator('.player-info strong').hover();
        assert.equal(await page.evaluate(() => hovered.filter(Boolean).length), 0);
        await card.locator('.player-score').hover();
        assert.equal(await page.evaluate(() => hovered.filter(Boolean).length), 0);
        await card.locator('.player-name').hover();
        assert.equal(await page.evaluate(() => hovered.at(-1)?.index), 0);
        await card.locator('.player-score').hover();
        assert.equal(await page.evaluate(() => hovered.at(-1)), null);
        await card.locator('.player-avatar').hover();
        assert.equal(await page.evaluate(() => hovered.at(-1)?.index), 0);
      } else {
        // The mobile decision panel hides the player strip; no hovercard is opened.
        assert.equal(await card.isVisible(), false);
        assert.equal(await page.evaluate(() => hovered.filter(Boolean).length), 0);
      }
      assert.deepEqual(await page.evaluate(() => sent), []);
      assert.deepEqual(errors, []);
      const bounds = await table.boundingBox();
      assert(bounds.x >= 0 && bounds.x + bounds.width <= width + 1);
      await page.close();
      console.log(
        `${locale} ${width}px: engaged units, fortress, conditional/active Steel Weapons, identity-only hover passed`,
      );
    }
} finally {
  await browser.close();
}
