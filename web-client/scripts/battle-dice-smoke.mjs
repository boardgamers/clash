import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';

// Mount the real live/replay popup with recorded dice, without an engine or timer.
const server = await createServer({
  configFile: false,
  appType: 'custom',
  root: new URL('..', import.meta.url).pathname,
  plugins: [svelte()],
  server: { host: '127.0.0.1', port: 0 },
});
server.middlewares.use(async (req, res, next) => {
  if (req.url !== '/battle-dice-test') return next();
  res.setHeader('Content-Type', 'text/html');
  res.end(
    await server.transformIndexHtml(
      req.url,
      `<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">
    <style>:root { --muted: #777; --ink: #222; --paper2: #fff; --line: #ccc; }</style>
    <div class="play-layout"><div class="live-battle" id="app"></div></div>
    <script type="module">
      import { mount } from 'svelte';
      import { writable } from 'svelte/store';
      import BattlePlayback from '/src/BattlePlayback.svelte';
      import '/src/combat.css';
      const session = writable({ battles: [] });
      window.showBattle = (dice, motion) => session.set({
        battles: [{ key: JSON.stringify(dice) + motion, roll: true, losses: [], combat: {
          round: 1, attacker: { dice, value: 7, hits: 1, modifiers: [] },
          defender: { dice: [], value: 0, hits: 0, modifiers: [] },
        } }], battleAnimate: motion, reducedMotion: !motion,
      });
      mount(BattlePlayback, { target: document.querySelector('#app'), props: {
        controller: { session, dismissBattles() { session.set({ battles: [] }); } },
      } });
    </script>`,
    ),
  );
});
await server.listen();
const browser = await chromium.launch({ headless: true });
try {
  for (const width of [1400, 390, 320]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/battle-dice-test`);
    await page.waitForFunction(() => typeof window.showBattle === 'function');
    for (const motion of [true, false]) {
      for (const count of [0, 1, 3]) {
        const dice = [
          ...Array.from({ length: count }, () => ({ value: 1, symbol: 'Leader', effect: 're-roll' })),
          { value: 6, symbol: 'Infantry', effect: '+1 combat value' },
          { value: 1, symbol: 'Leader' }, // No active leader: keep this face.
          { value: 1, symbol: 'Leader', blockedBy: 'High Ground' },
        ];
        await page.evaluate(({ dice, motion }) => window.showBattle(dice, motion), { dice, motion });
        const popup = page.getByRole('region', { name: 'Battle summary' });
        await popup.waitFor();
        assert.equal(await popup.locator('.battle-die').count(), dice.length);
        assert.deepEqual(
          await popup.locator('.battle-die b').allTextContents(),
          dice.map((d) => String(d.value)),
        );
        assert.equal(
          await popup.getByRole('img', { name: '1 · Leader face · Rerolled', exact: true }).count(),
          count,
        );
        assert.equal(await popup.locator('.rerolled').count(), count);
        assert.deepEqual(
          await popup
            .locator('.battle-die b')
            .evaluateAll((nodes) =>
              nodes.map((node) => getComputedStyle(node).textDecorationLine.includes('line-through')),
            ),
          dice.map((d) => d.effect === 're-roll'),
        );
        assert.equal(await popup.locator('.ability-blocked.rerolled').count(), 0);
        assert.equal(await popup.locator('.battle-hit-count').first().innerText(), '7 value\n1 hit');
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      }
    }
    await page.screenshot({ path: `/tmp/clash-battle-rerolls-${width}.png` });
    assert.deepEqual(errors, []);
    await page.getByRole('button', { name: 'Dismiss battle summary' }).click();
    assert.equal(await page.locator('.battle-recap').count(), 0);
    console.log(
      `${width}px: discarded rerolls crossed; final, inactive and blocked rolls retained once, with and without motion.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
  await server.close();
}
