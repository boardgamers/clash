// Local built viewer: npm run build; preview must be serving on localhost:8671.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const locale of ['en', 'fr']) {
    const catalog = JSON.parse(
      await readFile(new URL(`../src/localization/${locale}.json`, import.meta.url), 'utf8'),
    );
    for (const width of [1400, 390]) {
      const page = await browser.newPage({ viewport: { width, height: 900 }, hasTouch: true });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.goto(`http://localhost:8671/?locale=${locale}`);
      await page.waitForSelector('.age-track');
      await page.waitForSelector(`[data-clash-locale="${locale}"]`);
      async function emit(mode, age = 1, round = 1, triggered = false) {
        const game = JSON.parse(await engine.init(2, [], { length: mode }, 'age-track-smoke', {}));
        Object.assign(game, { age, round });
        if (triggered) game.variable_end = 'WonderfulEnding';
        const state = engine.stripSecret(JSON.stringify(game), game.current_player_index);
        await page.evaluate((state) => host.emit('state', state), state);
        await page.waitForTimeout(100);
      }
      for (const [mode, count, name] of [
        ['Shorter', 4, 'Shorter'],
        ['Standard', 6, 'Normal'],
        ['RollToEnd', 7, 'Roll to End'],
        ['WonderfulEnding', 7, 'Wonderful Ending'],
        ['Epic', 8, 'Epic'],
      ]) {
        await emit(mode, 1, 1);
        assert.equal(await page.locator('.age-step').count(), count);
        await page.locator('.age-step').last().tap();
        const details = page.locator('.age-length-details');
        await details.waitFor();
        assert.ok((await details.innerText()).includes(catalog[name]));
        assert.equal(await details.locator('footer').count(), 0);
        assert.ok(
          (await details.locator('strong').innerText()).includes(
            `${catalog['Age']} ${['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII'][count - 1]}`,
          ),
        );
        assert.doesNotMatch(await details.innerText(), /VI\.[123]|VII\.[123]/);
        const box = await details.boundingBox();
        assert.ok(box.x >= 0 && box.x + box.width <= width, `popover fits ${width}`);
        await page.keyboard.press('Escape');
        assert.equal(await details.count(), 0);
        assert.equal(
          await page
            .locator('.age-step')
            .last()
            .evaluate((node) => document.activeElement === node),
          true,
        );
      }
      await emit('RollToEnd', 6, 1);
      assert.equal(await page.locator('.age-phase').count(), 0);
      assert.equal(await page.locator('.age-step.current .inactive').count(), 1);
      for (const [age, round, threshold] of [
        [6, 2, '6'],
        [6, 3, '5–6'],
        [7, 1, '4–6'],
        [7, 2, '3–6'],
        [7, 3, '2–6'],
      ]) {
        await emit('RollToEnd', age, round);
        assert.equal(await page.locator('.age-phase').innerText(), threshold);
      }
      await page.locator('.age-step').nth(5).focus();
      await page.keyboard.press('Enter');
      const rows = page.locator('.age-end-thresholds tbody tr');
      assert.equal(await rows.count(), 5);
      assert.deepEqual(
        await rows.evaluateAll((nodes) =>
          nodes.map((node) => [...node.cells].map((cell) => cell.textContent)),
        ),
        [
          ['VI', '2', '6'],
          ['VI', '3', '5–6'],
          ['VII', '1', '4–6'],
          ['VII', '2', '3–6'],
          ['VII', '3', '2–6'],
        ],
      );
      assert.equal(await page.locator('.age-end-thresholds tr.current[aria-current="step"]').count(), 1);
      assert.deepEqual(await page.locator('.age-end-thresholds tr.current td').allTextContents(), [
        'VII',
        '3',
        '2–6',
      ]);
      await page.screenshot({ path: `/tmp/clash-age-roll-${locale}-${width}.png` });
      await page.locator('.age-details-close').click();
      for (const [age, round] of [
        [5, 3],
        [6, 1],
      ]) {
        await emit('WonderfulEnding', age, round);
        assert.equal(await page.locator('.age-phase').count(), 0);
      }
      for (const [age, round] of [
        [6, 2],
        [6, 3],
        [7, 1],
        [7, 2],
        [7, 3],
      ]) {
        await emit('WonderfulEnding', age, round);
        assert.equal(await page.locator('.age-phase').innerText(), catalog['Wonder to end']);
      }
      await emit('WonderfulEnding', 6, 2);
      await page.locator('.age-phase').tap();
      assert.ok(
        (await page.locator('.age-length-details').innerText()).includes(
          catalog[
            'Starting from age 6, round 2, building a wonder ends the game after that round. Then complete objectives.'
          ],
        ),
      );
      assert.equal(await page.locator('.age-length-details p').count(), 1);
      assert.equal(await page.locator('.age-length-details table, .age-length-details footer').count(), 0);
      assert.ok(
        await page.locator('.age-length-details').evaluate((node) => {
          const box = node.getBoundingClientRect();
          return node.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2));
        }),
        'details must render above the map action toolbar',
      );
      await page.screenshot({ path: `/tmp/clash-age-wonder-${locale}-${width}.png` });
      await page.locator('.brand').click(); // Outside click dismisses the anchored details.
      assert.equal(await page.locator('.age-length-details').count(), 0);
      await emit('WonderfulEnding', 6, 2, true);
      assert.equal(await page.locator('.age-phase').innerText(), catalog['Ends this round']);
      await page.locator('.age-phase').tap();
      assert.ok(
        (await page.locator('.age-length-details').innerText()).includes(
          catalog['Wonderful Ending: finish this round, then complete objectives'],
        ),
      );
      await page.keyboard.press('Escape');
      assert.equal(await page.locator('.length-rule').count(), 0);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert.deepEqual(errors, []);
      console.log(`Age-track tap/keyboard/round-window checks passed: ${locale}, ${width}px`);
      await page.close();
    }
  }
} finally {
  await browser.close();
}
