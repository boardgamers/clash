import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { startPreview } from './raiders-preview.mjs';
const server = await startPreview(0);
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const spectator of [true, false]) for (const width of [1400, 1047, 390]) {
    const height = width === 390 ? 740 : width === 1047 ? 827 : 900;
    const page = await browser.newPage({ viewport: { width, height }, hasTouch: width === 390 });
    const errors = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/?seat=${spectator ? 'spectator' : '1'}`);
    await page.locator('.player-card').first().waitFor();
    if (spectator) {
      await page.locator('.game-shell.spectating').waitFor();
      assert.equal(await page.locator('.board-actions').count(), 0);
      assert.equal(await page.locator('.cards-button, .wonders-button, .objectives-button').count(), 0);
      assert.doesNotMatch(await page.locator('.player-list').textContent(), /You/);
    }
    for (const name of ['Show regions', 'Show all sea routes', 'Show trade routes', 'Show pirate spawn positions', 'Show barbarian spawn and movement', 'Strategy map', 'Zoom in', 'Zoom out']) {
      const button = page.getByRole('button', { name, exact: true });
      assert(await button.isVisible(), `${name} is visible`);
      assert(await button.isEnabled(), `${name} is enabled`);
      const box = await button.boundingBox();
      assert(box.x >= 0 && box.y >= 0 && box.x + box.width <= width + 1 && box.y + box.height <= height + 1, `${name} fits viewport`);
    }
    await page.getByRole('button', { name: 'Show regions', exact: true }).click();
    await page.getByRole('combobox', { name: 'Region player' }).selectOption('1');
    await page.getByRole('button', { name: 'Show regions', exact: true }).click();
    for (const name of ['Show all sea routes', 'Show pirate spawn positions', 'Show barbarian spawn and movement', 'Zoom in', 'Zoom out']) {
      await page.getByRole('button', { name, exact: true }).click();
    }
    const toggle = page.getByRole('button', { name: 'Show trade routes', exact: true });
    if (width === 390) await toggle.tap();
    else { await toggle.focus(); await page.keyboard.press('Enter'); }
    await page.getByRole('combobox', { name: 'Trade route player' }).selectOption('1');
    const warning = page.locator('.raided-settlers-note');
    const guideWarning = page.locator('.raided-settlers-guide');
    assert.equal(await guideWarning.textContent(), 'Settlers blocked by Huns’ Raiders');
    assert.equal(await guideWarning.locator('svg').count(), 1);
    assert((await guideWarning.boundingBox()).height < 24, 'warning stays one compact line');
    assert.equal(await guideWarning.evaluate(node => getComputedStyle(node).borderLeftWidth), '0px');
    assert.equal(await page.locator('.collapsible-map-guide .coordinate-link').count(), 2, 'valid ship pairing remains');
    assert.doesNotMatch(await page.locator('.collapsible-map-guide').textContent(), /Current eligible pairings|unit #|warning only|does not change/i);
    assert.equal(await warning.count(), 0, 'no warning before tile inspection');
    // Use the normal coordinate link to locate the source, then physically hit
    // its rendered 3D hex (no event interception or controller test hooks).
    await page.locator('.collapsible-map-guide .coordinate-link').filter({ hasText: /^D7$/ }).click();
    await page.locator('.collapsible-map-guide summary').click();
    await page.waitForTimeout(300);
    const reference = await page.getByRole('status', { name: 'Map location D7' }).boundingBox();
    assert(reference, 'D7 has a visible projected map reference');
    const point = { x: reference.x + reference.width / 2, y: reference.y - 8 };
    if (width === 390) await page.touchscreen.tap(point.x, point.y);
    else await page.mouse.click(point.x, point.y);
    await warning.waitFor();
    assert.equal(await warning.textContent(), 'Settlers blocked by Huns’ Raiders');
    await page.getByRole('button', { name: spectator ? 'Close tile details' : 'Close tile actions', exact: true }).click();
    for (const strategy of [false, true]) {
      if (strategy) await page.getByRole('button', { name: 'Strategy map', exact: true }).click();
      if (strategy) {
        // Strategy changes camera orientation; re-center through the real guide
        // before tapping a tile so the collapsed guide cannot cover the target.
        await page.locator('.collapsible-map-guide summary').click();
        await page.locator('.collapsible-map-guide .coordinate-link').filter({ hasText: /^D7$/ }).click();
        await page.locator('.collapsible-map-guide summary').click();
        const tile = page.locator('.strategy-map-tile[data-position="D7"]');
        if (width === 390) await tile.tap(); else await tile.click();
        await warning.waitFor();
        assert.equal(await warning.textContent(), 'Settlers blocked by Huns’ Raiders');
        await page.getByRole('button', { name: spectator ? 'Close tile details' : 'Close tile actions', exact: true }).click();
        await tile.focus();
        await page.keyboard.press('Enter');
        await warning.waitFor();
        assert.match(await warning.textContent(), /Settlers blocked by Huns’ Raiders/);
        await page.screenshot({ path: `/tmp/clash-raiders-click-${spectator}-${width}-${strategy}.png` });
        await page.getByRole('button', { name: spectator ? 'Close tile details' : 'Close tile actions', exact: true }).click();
        assert.equal(await warning.count(), 0);
      }
      for (const dark of [true, false]) {
        await page.evaluate(dark => host.emit('theme', { dark }), dark);
        await page.waitForTimeout(700);

        await page.screenshot({ path: `/tmp/clash-raiders-${spectator ? 'spectator' : 'player'}-${width}-${strategy ? 'strategy' : '3d'}-${dark ? 'dark' : 'light'}.png` });
      }
    }
    await page.locator('.collapsible-map-guide summary').click();
    await page.getByRole('combobox', { name: 'Trade route player' }).selectOption('0');
    assert.equal(await warning.count(), 0);
    assert.equal(await guideWarning.count(), 0);
    await page.getByRole('combobox', { name: 'Trade route player' }).selectOption('1');
    assert.equal(await warning.count(), 0);
    assert.equal(await guideWarning.textContent(), 'Settlers blocked by Huns’ Raiders');
    const layout = await page.evaluate(() => {
      const board = document.querySelector('.play-layout');
      const map = document.querySelector('.map-section');
      const firstPlayer = document.querySelector('.player-card');
      return { scroll: board.scrollTop, board: board.getBoundingClientRect().toJSON(), map: map.getBoundingClientRect().toJSON(), player: firstPlayer.getBoundingClientRect().toJSON() };
    });
    assert.equal(layout.scroll, 0, 'focusing guides must not scroll the board under the header');
    assert(Math.abs(layout.map.bottom - layout.board.bottom) < 1, 'map fills the board, not the whole viewport');
    assert(layout.player.top >= layout.board.top, 'player cards stay below the header');
    await toggle.click();
    assert.equal(await warning.count(), 0);
    assert.deepEqual(await page.evaluate(() => sent), []);
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log('Raiders: physical 3D/Strategy clicks, desktop keyboard/mobile touch; dark/light; spectator/off-turn civilization switch; visible map toolbar; no moves submitted.');
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
