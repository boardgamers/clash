import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let initial = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'leader-ui', {});
for (const civ of ['China', 'Greece'])
  initial = engine.tryMove(
    initial,
    JSON.stringify({ ChooseCivilization: civ }),
    engine.currentPlayer(initial),
  );
initial = JSON.parse(initial);
const seat = engine.currentPlayer(JSON.stringify(initial));
initial.players[seat].cities[0].mood_state = 'Happy';
initial.players[seat].resources = { mood_tokens: 1, culture_tokens: 1 };
initial.players[seat].action_cards = [7, 11, 17];
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390, 320]) {
    const page = await browser.newPage({ viewport: { width, height: width === 320 ? 640 : 900 } });
    let state = structuredClone(initial);
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, seat));
      await page.evaluate(
        (state) => host.emit('state', state),
        engine.stripSecret(JSON.stringify(state), seat),
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
    await page.goto('http://clash.test/');
    await page.evaluate(
      ({ state, seat }) => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: engine.stripSecret(JSON.stringify(state), seat), seat },
    );
    const cityButton = page.getByRole('button', { name: 'Manage cities', exact: true });
    await cityButton.waitFor();
    assert.ok(!(await cityButton.getAttribute('class')).includes('inspect-only'));
    await cityButton.click();
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    const sunTzu = page.getByRole('button', { name: 'Select Sun Tzu', exact: true });
    assert.ok(await sunTzu.isEnabled());
    await sunTzu.click();
    await page.screenshot({ path: `/tmp/clash-leader-available-${width}.png` });
    assert.ok(await page.locator('.city-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.getByRole('button', { name: 'Close city management' }).click();
    await page.getByRole('button', { name: 'Action cards: 3', exact: true }).click();
    await page
      .getByText('Attacking only · City battle · Requires your Army in the battle.', { exact: false })
      .waitFor();
    assert.ok(await page.locator('.cards-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-card-rules-${width}.png` });
    await page.getByRole('button', { name: 'Close action cards' }).click();
    state.players[seat].resources = {};
    await page.evaluate(
      (state) => host.emit('state', state),
      engine.stripSecret(JSON.stringify(state), seat),
    );
    await page.waitForFunction(() =>
      document.querySelector('[aria-label="Manage cities"]')?.classList.contains('inspect-only'),
    );
    await cityButton.click();
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    assert.ok(!(await page.getByRole('button', { name: 'Select Sun Tzu', exact: true }).isEnabled()));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: leader availability, eye icon, unavailable recruitment and card restrictions verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
