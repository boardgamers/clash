import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'tactics-ui', {})).players.slice(2);
async function attacked(cards) {
  const game = JSON.parse(
    await readFile(
      new URL('../../server/tests/test_games/tactics_cards/peltasts.json', import.meta.url),
      'utf8',
    ),
  );
  game.players.push(...structuredClone(npcs));
  game.players[0].action_cards = [];
  game.players[1].action_cards = cards;
  game.players[1].resources.culture_tokens = 0;
  return engine.tryMove(
    JSON.stringify(game),
    JSON.stringify({ Movement: { Move: { units: [0], destination: 'C1', payment: {} } } }),
    0,
  );
}
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
    [320, 640],
  ]) {
    for (const variant of ['single', 'multiple', 'skip']) {
      let state = await attacked(variant === 'multiple' ? [4, 3, 7, 9, 11] : [4]);
      const original = JSON.parse(state),
        sent = [],
        errors = [];
      const page = await browser.newPage({ viewport: { width, height } });
      page.on('pageerror', (e) => errors.push(e.message));
      await page.exposeFunction('applyMove', async (action) => {
        sent.push(JSON.parse(action));
        state = engine.tryMove(state, action, 1);
        await page.evaluate(
          (state) => {
            host.emit('state', state);
            window.moveApplied = true;
          },
          engine.stripSecret(state, 1),
        );
      });
      await page.route('**/*', async (route) => {
        const path = new URL(route.request().url()).pathname;
        if (path === '/')
          return route.fulfill({
            contentType: 'text/html',
            body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
          });
        if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
        return route.fulfill({
          contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
          body: await readFile(new URL('../dist' + path, import.meta.url)),
        });
      });
      await page.goto('http://clash.test');
      await page.evaluate(
        (state) => {
          window.host = clash3d.launch('#app');
          host.on('move', (a) => window.applyMove(a));
          host.emit('player', { index: 1 });
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false });
          host.emit('state', state);
        },
        engine.stripSecret(state, 1),
      );
      const panel = page.getByRole('region', { name: 'Tactics', exact: true });
      await panel.waitFor();
      const peltasts = panel.getByRole('button', { name: 'Select Peltasts', exact: true });
      await peltasts.waitFor();
      assert.match(await peltasts.innerText(), /Roll a die for each of your Army units/);
      assert.doesNotMatch(await peltasts.innerText(), /Gain 1 advance/);
      assert.equal(await panel.locator('.decision-count').count(), 0);
      const info = panel
        .locator('.tactics-choice')
        .filter({ has: page.getByRole('button', { name: 'Select Peltasts', exact: true }) })
        .locator('details');
      assert.equal(await info.getAttribute('open'), null);
      assert.equal(await info.locator('p').isVisible(), false);
      await info.locator('summary').click();
      assert.match(await info.innerText(), /Unavailable during battle/);
      assert.match(await info.innerText(), /Gain 1 advance/);
      assert.equal(await peltasts.getAttribute('aria-pressed'), 'false');
      assert.equal(sent.length, 0, 'Reading the civil action must not select or play it');
      await info.locator('summary').click();
      await peltasts.click();
      assert.equal(await peltasts.getAttribute('aria-pressed'), 'true');
      const play = panel.getByRole('button', { name: 'Play tactics', exact: true });
      assert(await play.isEnabled());
      assert.equal(sent.length, 0);
      assert(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      if (width < 760) {
        const bounds = await panel.boundingBox();
        assert(bounds.height <= 400 && bounds.height < height * 0.6);
        assert(bounds.y >= 100 && bounds.y + bounds.height <= height - 55);
        assert((await play.boundingBox()).width < 170);
      }
      await play.scrollIntoViewIfNeeded();
      await page.screenshot({ path: `/tmp/clash-tactics-${variant}-${width}.png` });
      await (variant === 'skip' ? panel.getByRole('button', { name: 'Skip', exact: true }) : play).click();
      await page.waitForFunction(() => window.moveApplied);
      assert.deepEqual(sent, [
        { Response: { SelectHandCards: variant === 'skip' ? [] : [{ ActionCard: 4 }] } },
      ]);
      const after = JSON.parse(state);
      assert.deepEqual(after.players[1].advances, original.players[1].advances);
      assert.equal(after.players[1].resources.culture_tokens ?? 0, 0);
      assert.equal((after.players[1].action_cards ?? []).includes(4), variant === 'skip');
      assert.deepEqual(errors, []);
      await page.close();
    }
    console.log(
      `${width}px: battle effects first, optional civil reference, compact single/multiple cards, play and skip verified.`,
    );
  }
} finally {
  await browser.close();
}
