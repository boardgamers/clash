// Production viewer/controller and actual WASM battle; only BGS transport is local.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { battle, engine, items } from './horse-master-fixture.mjs';
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    let raw = battle();
    const originalRolls = items(JSON.parse(raw)).filter((i) => i.CombatRoll);
    assert.deepEqual(
      originalRolls.map((i) => i.CombatRoll.combat_value),
      [12, 12],
    );
    const emit = () => page.evaluate((state) => host.emit('state', state), engine.stripSecret(raw, 1));
    const sent = [];
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      raw = engine.tryMove(raw, action, 1);
      await emit();
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
    await page.waitForFunction(() => typeof window.clash3d !== 'undefined');
    await page.evaluate(() => {
      window.host = clash3d.launch('#app');
      host.on('move', (action) => window.applyMove(action));
      host.emit('player', { index: 1 });
      host.emit('preferences', { sound: false, reducedMotion: false });
    });
    await emit();
    await page
      .getByText(/Horse Master/)
      .first()
      .waitFor();
    await page.getByRole('button', { name: 'Cancel 1 hit', exact: true }).click();
    await page.waitForTimeout(500);
    assert.deepEqual(sent[0], { Response: { Bool: true } });
    const state = JSON.parse(raw);
    assert.deepEqual(
      items(state).filter((i) => i.CombatRoll),
      originalRolls,
      'no new dice roll',
    );
    assert.deepEqual(
      items(state)
        .filter((i) => i.CombatHitReport)
        .slice(-2)
        .map((i) => i.CombatHitReport),
      [
        { combat_value: 12, hit_cancels: 1, extra_hits: 0, casualties: 1 },
        { combat_value: 8, hit_cancels: 0, extra_hits: 0, casualties: 1 },
      ],
    );
    const sides = page.locator('.battle-side');
    assert.deepEqual(await page.locator('.battle-hit-count > span').allTextContents(), [
      '12 value',
      '8 value',
    ]);
    assert.match(await sides.nth(0).locator('.battle-hit-count strong').innerText(), /1 hit$/);
    assert.equal(await sides.nth(1).locator('.battle-hit-count strong').innerText(), '1 hit');
    assert.deepEqual(await sides.nth(0).locator('.battle-die b').allTextContents(), ['5', '2', '5']);
    assert.deepEqual(await sides.nth(1).locator('.battle-die b').allTextContents(), ['2', '6']);
    assert.equal(await page.locator('.battle-roll').count(), 0, 'conversion must not animate another roll');
    await page.getByRole('button', { name: 'Cavalry', exact: true }).click();
    await page.getByRole('button', { name: 'Confirm', exact: true }).click();
    await page.waitForTimeout(300);
    assert.equal(JSON.parse(raw).players[0].units.length, 1);
    assert.equal(JSON.parse(raw).players[1].units.length, 1);
    assert.equal(JSON.parse(raw).events.at(-1).handler.request.BoolRequest, 'Do you want to retreat?');
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    await page.close();
    console.log(
      `${width}px: actual Horse Master accepted through production viewer; final CV 12/8, one casualty each, unchanged rolls.`,
    );
  }
} finally {
  await browser.close();
}
