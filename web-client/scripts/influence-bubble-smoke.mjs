import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { startPreview } from './influence-preview.mjs';

const server = await startPreview(0);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});

function boxesOverlap(a, b) {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}

try {
  const baseUrl = `http://127.0.0.1:${server.address().port}`;

  // 1. Standard 3D view: uninfluenced, 1-building, multi-building count pip, multi-civ, click-to-select, no overlap.
  {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(`${baseUrl}/`);
    await page.locator('.city-influence-bubble').first().waitFor();
    await page.waitForTimeout(400);

    const bubbles = page.locator('.city-influence-bubble');
    assert.equal(await bubbles.count(), 3, 'Uninfluenced C3 has no bubble; F3, F5, C5 each have one');

    const bubbleInfo = await page.evaluate(() =>
      [...document.querySelectorAll('.city-influence-bubble')].map((el) => ({
        title: el.getAttribute('title') ?? '',
        ariaLabel: el.getAttribute('aria-label') ?? '',
        text: (el.innerText ?? '').trim(),
        medallions: el.querySelectorAll('.influence-medallion').length,
        counts: [...el.querySelectorAll('.influence-count')].map((n) => n.textContent?.trim()),
        symbols: el.querySelectorAll('.influence-symbol').length,
        rect: el.getBoundingClientRect().toJSON(),
      })),
    );

    const single = bubbleInfo.find((b) => b.title === 'Influenced by Greece');
    const multiCount = bubbleInfo.find((b) => b.title === 'Influenced by Greece (3)');
    const multiCiv = bubbleInfo.find((b) => b.title === 'Influenced by Greece, Egypt');
    assert.ok(single, 'F3 single-building influence bubble present');
    assert.equal(single.medallions, 1);
    assert.deepEqual(single.counts, []);
    assert.equal(single.symbols, 0);

    assert.ok(multiCount, 'F5 multi-building influence bubble present');
    assert.equal(multiCount.medallions, 1);
    assert.deepEqual(multiCount.counts, ['3']);

    assert.ok(multiCiv, 'C5 multi-civ influence bubble present');
    assert.equal(multiCiv.medallions, 2);
    assert.deepEqual(multiCiv.counts, []);

    for (const b of bubbleInfo) {
      assert.doesNotMatch(b.text, /Greece|Egypt|Rome/i, 'No visible civilization text on board bubbles');
    }

    // Verify no overlap between any city mood badge and any influence bubble in 3D view.
    const moodRects = await page.evaluate(() =>
      [...document.querySelectorAll('.city-map-label')].map((el) => ({
        label: el.getAttribute('aria-label') ?? '',
        rect: el.getBoundingClientRect().toJSON(),
      })),
    );
    assert.equal(moodRects.length, 4, 'All 4 cities render mood badges');
    for (const bubble of bubbleInfo) {
      for (const mood of moodRects) {
        assert.equal(
          boxesOverlap(bubble.rect, mood.rect),
          false,
          `Bubble "${bubble.title}" must not overlap mood badge "${mood.label}"`,
        );
      }
    }

    // Clicking a thought bubble selects its city without submitting any gameplay moves.
    await page.locator('.city-influence-bubble[title="Influenced by Greece (3)"]').click();
    await page.getByRole('region', { name: /Tile F5/ }).waitFor();
    assert.deepEqual(await page.evaluate(() => window.sent), [], 'Clicking influence bubble must not send moves');
    assert.deepEqual(errors, []);
    await page.close();
  }

  // 2. Strategy view (normal + color-blind): compact bubbles, zero civ text on board, color-blind symbols centered.
  for (const colorBlind of [false, true]) {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(`${baseUrl}/?strategy=1${colorBlind ? '&colorblind=1' : ''}`);
    await page.locator('.strategy-map-tile[data-position="F5"]').waitFor();

    assert.equal(
      await page.locator('.strategy-map-tile[data-position="C3"] .strategy-influence-bubble').count(),
      0,
      'Uninfluenced C3 has no Strategy influence bubble',
    );
    const f3 = page.locator('.strategy-map-tile[data-position="F3"] .strategy-influence-bubble');
    const f5 = page.locator('.strategy-map-tile[data-position="F5"] .strategy-influence-bubble');
    const c5 = page.locator('.strategy-map-tile[data-position="C5"] .strategy-influence-bubble');
    assert.equal(await f3.getAttribute('title'), 'Influenced by Greece');
    assert.equal(await f5.getAttribute('title'), 'Influenced by Greece (3)');
    assert.equal(await c5.getAttribute('title'), 'Influenced by Greece, Egypt');
    assert.equal(await f3.locator('.influence-medallion').count(), 1);
    assert.equal(await f3.locator('.influence-count').count(), 0);
    assert.equal(await f5.locator('.influence-count').textContent(), '3');
    assert.equal(await c5.locator('.influence-medallion').count(), 2);

    for (const tilePos of ['C3', 'F3', 'F5', 'C5']) {
      const tileText = await page.locator(`.strategy-map-tile[data-position="${tilePos}"]`).innerText();
      assert.doesNotMatch(tileText, /Greece|Egypt|Rome/i, `Strategy tile ${tilePos} has no visible civ text`);
    }

    if (colorBlind) {
      assert.equal(await f3.locator('.influence-symbol[data-symbol="▲"] svg.ownership-symbol-svg').count(), 1);
      assert.equal(await c5.locator('.influence-symbol[data-symbol="▲"] svg.ownership-symbol-svg').count(), 1);
      assert.equal(await c5.locator('.influence-symbol[data-symbol="■"] svg.ownership-symbol-svg').count(), 1);
      assert.ok((await page.locator('.strategy-symbol svg.ownership-symbol-svg').count()) >= 4);
      assert.equal(await page.locator('.ownership-symbol svg.ownership-symbol-svg').count(), 3);
    } else {
      assert.equal(await page.locator('.influence-symbol').count(), 0);
    }

    assert.deepEqual(await page.evaluate(() => window.sent), []);
    assert.deepEqual(errors, []);
    await page.close();
  }

  // 3. 3D color-blind mode: centered SVG pills in thought bubbles + player cards, zero errors.
  {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(`${baseUrl}/?colorblind=1`);
    await page.locator('.city-influence-bubble').first().waitFor();
    await page.waitForTimeout(400);

    assert.equal(await page.locator('.city-influence-bubble .influence-symbol svg.ownership-symbol-svg').count(), 4);
    assert.equal(await page.locator('.ownership-symbol svg.ownership-symbol-svg').count(), 3);
    assert.deepEqual(await page.evaluate(() => window.sent), []);
    assert.deepEqual(errors, []);
    await page.close();
  }

  console.log(
    'Cultural influence bubbles & centered color-blind symbols: 3D/Strategy, single/counted/multi-civ, click-to-select, zero civ text, no mood overlap, and color-blind mode passed.',
  );
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
