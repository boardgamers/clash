import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { unexploredRegion } from '../src/exploration-preview.ts';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const initial = await engine.init(2, [], { civilization: 'Random' }, 'exploration-preview', {});
const seat = engine.currentPlayer(initial);
const game = JSON.parse(engine.stripSecret(initial, seat));
const view = JSON.parse(engine.webView(JSON.stringify(game), seat));
const settler = view.settlers[0];
const targets = settler.destinations.filter((d) => d.terrain === 'Unexplored');
const land = settler.destinations.find((d) => d.terrain === 'Forest');
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 780],
    [320, 640],
  ]) {
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [],
      sent = [];
    let state = initial;
    page.on('pageerror', (e) => errors.push(e.message));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = engine.tryMove(state, action, seat);
      await page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(state, seat));
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
      ({ seat, state }) => {
        window.host = clash3d.launch('#app');
        host.on('move', (action) => window.applyMove(action));
        host.emit('player', { index: seat });
        host.emit('theme', { dark: true });
        host.emit('preferences', { sound: false, unitBadges: true });
        host.emit('state', state);
      },
      { seat, state: JSON.stringify(game) },
    );
    const choose = async (position) => {
      const hex = page.locator(`.map-hit-target[data-position="${position}"]`);
      await hex.focus();
      await hex.press('Enter');
    };
    const open = async () => {
      await page.locator(`.unit-map-label[aria-label*="${settler.position}:"]`).click();
      const unit = page
        .getByRole('region', { name: 'Unit movement', exact: true })
        .getByRole('button', { name: 'Settler', exact: true });
      if ((await unit.getAttribute('aria-pressed')) !== 'true') await unit.click();
    };
    await open();
    const label = page.locator('.map-exploration-label');
    assert(await label.isHidden());
    for (const destination of targets.slice(0, 2)) {
      await choose(destination.position);
      await label.getByText('Will reveal', { exact: true }).waitFor();
      assert.deepEqual(
        (await label.getAttribute('data-positions')).split(' ').sort(),
        unexploredRegion(game.map, destination.position).sort(),
      );
      assert.deepEqual(sent, [], 'preview must not submit or reveal anything');
      assert.equal(state, initial);
    }
    await page.screenshot({ path: `/tmp/clash-exploration-preview-${width}.png` });
    await choose(land.position);
    assert(await label.isHidden(), 'explored destination clears outline');
    await choose(targets[0].position);
    await page.getByRole('button', { name: 'Close movement controls', exact: true }).click();
    assert(await label.isHidden(), 'cancelling clears outline');
    await open();
    await choose(targets[0].position);
    await page.getByRole('button', { name: /^Explore/, exact: false }).click();
    await page.waitForFunction(() => !document.querySelector('.settler-confirm'));
    assert.equal(sent.length, 1);
    assert.notEqual(state, initial, 'confirmation submits the actual move');
    const nextView = JSON.parse(engine.webView(engine.stripSecret(state, seat), seat));
    if (nextView.explorationDecision) assert.equal(await label.innerText(), 'Exploring');
    else assert(await label.isHidden());
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: four-hex preview before confirmation, destination changes, cancellation and real exploration verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
