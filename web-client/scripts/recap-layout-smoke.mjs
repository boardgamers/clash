import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const engine = createRequire(import.meta.url)('../.engine/server.js');
const dist = new URL('../dist/', import.meta.url);

// Only repository test positions are used. Execute the recorded battle with the
// actual engine, then group its payment/tactics responses into one public action.
// The synthetic preceding end-turn boundary makes this a returning-player recap.
export async function createFixture(stress = false) {
  const load = async (name) =>
    JSON.parse(
      await readFile(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8'),
    );
  const initial = await load('combat_all_modifiers');
  const npcs = JSON.parse(await engine.init(2, [], {}, 'recap-layout-smoke', {})).players.slice(2);
  for (const npc of npcs)
    if (!initial.players.some((p) => p.civilization === npc.civilization)) {
      initial.players.push({ ...npc, id: initial.players.length });
    }
  const outcome = await load('combat_all_modifiers.outcome5');
  const actions = outcome.log.flatMap((a) =>
    a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])),
  );
  let raw = JSON.stringify(initial);
  for (const action of actions) raw = engine.tryMove(raw, JSON.stringify(action.action), action.player);
  const battleCursor = JSON.parse(raw).board_history.frames.at(-1).cursor;
  raw = engine.tryMove(raw, JSON.stringify({ Movement: 'Stop' }), 0);
  raw = engine.tryMove(raw, JSON.stringify({ Playing: 'EndTurn' }), 0);
  const game = JSON.parse(engine.stripSecret(raw, 1));
  const frames = game.board_history.frames;
  const before = { ...frames[0], actor: 1, ended_turn: true, title: 'End turn' };
  const battle = { ...frames.find((f) => f.cursor === battleCursor), actor: 0, title: 'Battle' };
  const end = frames.at(-1);
  game.board_history = { id: `recap-layout-${stress ? 'stress' : 'battle'}`, frames: [before, battle, end] };
  if (stress) {
    // Deliberately overfull public notices exercise the content scroller, without
    // DOM injection or production/controller test hooks. Rules are real catalog data.
    const catalog = JSON.parse(engine.webView(raw, 1)).eventCatalog;
    const unique = [...new Map(catalog.map((e) => [e.name, e])).values()]
      .sort((a, b) => b.rules.join('').length - a.rules.join('').length)
      .slice(0, 12);
    const publicActions = game.log.flatMap((a) =>
      a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])),
    );
    publicActions[battleCursor - 1].items.push(
      ...unique.map((event) => ({
        player: 0,
        Text: `triggers the event ${event.name}`,
        origin: { Incident: event.id },
      })),
    );
  }
  return { raw: JSON.stringify(game), seat: 1, battleCursor, endCursor: end.cursor, stress };
}

export async function startPreview(port = 0) {
  const fixtures = { battle: await createFixture(), stress: await createFixture(true) };
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://localhost');
      if (url.pathname === '/fixture.json') {
        res.setHeader('Content-Type', 'application/json');
        return res.end(JSON.stringify(fixtures[url.searchParams.has('stress') ? 'stress' : 'battle']));
      }
      if (url.pathname === '/') {
        res.setHeader('Content-Type', 'text/html; charset=utf-8');
        return res.end(`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Recap layout · synthetic battle preview</title></head><body><div id="app"></div><script src="/viewer.js"></script><script>
(async () => {
  const params = new URLSearchParams(location.search);
  const fixture = await (await fetch('/fixture.json' + (params.has('stress') ? '?stress' : ''))).json();
  for (const key of Object.keys(localStorage)) if (key.startsWith('clash:board-seen:')) localStorage.removeItem(key);
  window.sent = []; window.replayInfo = []; window.fixture = fixture;
  window.host = clash3d.launch('#app');
  let prefs = { sound: false, replayAutoplay: false, locale: params.get('locale') || 'en' };
  host.on('move', (move) => sent.push(move));
  host.on('replay:info', (info) => replayInfo.push(info));
  host.on('update:preference', ({name, value}) => { prefs = {...prefs, [name]: value}; host.emit('preferences', prefs); });
  host.emit('player', { index: fixture.seat });
  host.emit('theme', { dark: params.get('theme') !== 'light' });
  host.emit('preferences', prefs); host.emit('state', fixture.raw);
  // Land on the recorded dice, not an empty start frame. Real controls remain live.
  if (!params.has('start')) {
    const timer = setInterval(() => {
      const next = document.querySelector('.playback-next');
      if (next && !next.disabled) { clearInterval(timer); next.click(); window.previewReady = true; }
    }, 40);
    setTimeout(() => clearInterval(timer), 20000);
  } else window.previewReady = true;
})();
</script></body></html>`);
      }
      const asset = new URL('.' + url.pathname, dist);
      if (!asset.href.startsWith(dist.href)) return res.writeHead(404).end();
      const types = {
        wasm: 'application/wasm',
        js: 'text/javascript',
        json: 'application/json',
        woff2: 'font/woff2',
        svg: 'image/svg+xml',
      };
      res.setHeader('Content-Type', types[url.pathname.split('.').at(-1)] ?? 'application/octet-stream');
      res.end(await readFile(asset));
    } catch (error) {
      res.writeHead(404).end(String(error));
    }
  });
  await new Promise((resolve) => server.listen(port, '127.0.0.1', resolve));
  return { server, url: `http://localhost:${server.address().port}/` };
}

async function assertLayout(page, width, height, stress = false) {
  const metrics = await page.locator('.playback-bar').evaluate((bar) => {
    const rect = (el) => {
      const b = el.getBoundingClientRect();
      return { x: b.x, y: b.y, width: b.width, height: b.height, right: b.right, bottom: b.bottom };
    };
    const scroller = bar.querySelector('.playback-content');
    return {
      bar: rect(bar),
      board: rect(document.querySelector('main.play-layout')),
      scrollWidth: bar.scrollWidth,
      clientWidth: bar.clientWidth,
      documentWidth: document.documentElement.scrollWidth,
      controls: [
        ...bar.querySelectorAll('.playback-transport button, .playback-exit, .playback-preference'),
      ].map((el) => ({
        ...rect(el),
        label: el.textContent,
        inScroller: !!scroller?.contains(el),
      })),
      scroller: scroller && {
        ...rect(scroller),
        overflow: getComputedStyle(scroller).overflowY,
        scrollHeight: scroller.scrollHeight,
        clientHeight: scroller.clientHeight,
      },
      centerCovered: !!document.elementFromPoint(innerWidth / 2, innerHeight / 2)?.closest('.playback-bar'),
    };
  });
  const b = metrics.bar;
  assert(
    b.x >= 0 && b.right <= width + 1 && b.y >= 0 && b.bottom <= height + 1,
    `panel is in viewport: ${JSON.stringify(metrics)}`,
  );
  assert(metrics.scrollWidth <= metrics.clientWidth + 1, 'no horizontal panel overflow');
  assert(metrics.documentWidth <= width, 'no horizontal document overflow');
  if (width >= 1100 || (width >= 760 && height <= 550)) {
    assert(b.x <= 30, `desktop panel stays at left, not center: x=${b.x}`);
    assert(b.width <= 380, `compact desktop width: ${b.width}`);
    assert(b.right < width / 2, 'desktop map center remains outside the dock');
    assert(!metrics.centerCovered, 'desktop map center is not covered');
    assert(
      b.y >= metrics.board.y && b.y <= metrics.board.y + 30,
      `dock starts near board area: y=${b.y}, board=${metrics.board.y}`,
    );
  } else {
    assert(
      b.bottom >= height - 150 && b.y >= 0,
      `mobile retains bottom panel above reserved toolbar: bottom=${b.bottom}`,
    );
  }
  assert(metrics.controls.length >= 5, 'Back, Next, Play/Replay, preference and Skip/Back to game exist');
  for (const control of metrics.controls) {
    assert(
      control.x >= b.x - 1 &&
        control.right <= b.right + 1 &&
        control.y >= b.y - 1 &&
        control.bottom <= b.bottom + 1,
      `control fits inside visible panel: ${JSON.stringify(control)}`,
    );
    assert(!control.inScroller, `control remains outside scrolling content: ${control.label}`);
  }
  if (stress) {
    assert(metrics.scroller, 'recap has dedicated content scroller');
    assert(['auto', 'scroll'].includes(metrics.scroller.overflow), 'overflow content can scroll');
    assert(
      metrics.scroller.scrollHeight > metrics.scroller.clientHeight + 20,
      'stress fixture actually overflows content',
    );
    const positions = metrics.controls.map(({ x, y }) => ({ x, y }));
    await page.locator('.playback-content').evaluate((el) => {
      el.scrollTop = el.scrollHeight;
    });
    assert(await page.locator('.playback-content').evaluate((el) => el.scrollTop > 0), 'content scrolls');
    const after = await page.locator('.playback-bar').evaluate((bar) =>
      [...bar.querySelectorAll('.playback-transport button, .playback-exit, .playback-preference')].map(
        (el) => {
          const r = el.getBoundingClientRect();
          return { x: r.x, y: r.y };
        },
      ),
    );
    assert.deepEqual(after, positions, 'scrolling content does not move any controls');
  }
  return b;
}

async function smoke() {
  const fixtureOnly = process.argv.includes('--fixture-only');
  const { server, url } = await startPreview();
  const browser = await chromium.launch({
    headless: true,
    args: ['--use-gl=angle', '--use-angle=swiftshader'],
  });
  const sizes = fixtureOnly
    ? [[1400, 850]]
    : [
        [1400, 850],
        [1920, 1080],
        [1100, 650],
        [1366, 520],
        [1099, 650],
        [844, 390],
        [390, 844],
        [320, 640],
      ];
  try {
    for (const [width, height] of sizes) {
      if (process.env.SMOKE_WIDTH && width !== Number(process.env.SMOKE_WIDTH)) continue;
      const page = await browser.newPage({ viewport: { width, height } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.goto(url);
      await page.waitForFunction(
        () => window.previewReady && document.querySelectorAll('.playback-bar .battle-die').length > 0,
      );
      const bar = page.locator('.playback-bar');
      const battle = bar.locator('.battle-recap');
      const next = bar.locator('.playback-next');
      const back = bar.locator('.playback-transport button').first();
      const play = bar.locator('.playback-transport button').nth(2);
      assert((await battle.locator('.battle-die').count()) >= 6, 'recorded dice render in recap');
      assert((await battle.locator('.battle-die svg').count()) >= 6, 'recorded dice retain unit faces');
      if (!fixtureOnly) await assertLayout(page, width, height);
      await page.waitForTimeout(1250); // Capture completed recorded dice, not their entrance animation.
      await page.screenshot({ path: `/tmp/clash-recap-dock-${width}x${height}.png` });
      await back.click();
      await battle.waitFor({ state: 'hidden' });
      assert(await back.isDisabled(), 'Back reaches the recorded start');
      await next.click();
      await battle.waitFor();
      await play.click();
      assert.match(await play.getAttribute('aria-label'), /Pause/, 'Play starts the recap');
      await play.click();
      assert.match(await play.getAttribute('aria-label'), /Play/, 'Pause stops the recap');
      await battle.locator('button').click();
      await battle.waitFor({ state: 'hidden' });
      await back.click();
      await next.click();
      await battle.waitFor();
      if (width === 1400) {
        await play.click();
        await page.waitForFunction(() => document.querySelector('.playback-next')?.disabled, null, {
          timeout: 15000,
        });
      } else await next.click();
      assert(await next.isDisabled(), 'manual Next/autoplay reaches bounded end');
      await play.click();
      assert(await back.isDisabled(), 'Replay returns to start');
      await bar.locator('.playback-exit').click();
      await bar.waitFor({ state: 'hidden' });
      await page.locator('.last-turn-button').click();
      await bar.waitFor();
      await next.click();
      await battle.waitFor();
      if (!fixtureOnly) await assertLayout(page, width, height);
      // Native host full replay must not inherit the recap dock or controls.
      await page.evaluate(() => {
        host.emit('replay:start');
        host.emit('replay:to', fixture.battleCursor);
      });
      await bar.waitFor({ state: 'hidden' });
      await page.locator('.battle-recap').waitFor();
      await page.waitForFunction(() => replayInfo.at(-1)?.current === fixture.battleCursor);
      assert.equal(
        await page.locator('.playback-bar').count(),
        0,
        'full replay does not render recap controls',
      );
      await page.setViewportSize({ width: width >= 1100 ? 390 : 1400, height: 850 });
      assert.equal(
        await page.locator('.playback-bar').count(),
        0,
        'resizing full replay does not create recap dock',
      );
      await page.evaluate(() => host.emit('replay:end'));
      await page.setViewportSize({ width, height });
      await page.goto(url + '?stress&locale=fr');
      await page.waitForFunction(
        () => window.previewReady && document.querySelectorAll('.replay-event-explanations').length > 0,
      );
      await page.locator('.playback-bar .battle-recap').waitFor();
      await page.waitForFunction(() =>
        document.querySelector('.playback-exit')?.textContent.includes('Passer'),
      );
      assert(
        (await bar.locator('.replay-event-explanations').innerText()).length > 500,
        'many real event rules populate overflow fixture',
      );
      if (!fixtureOnly) await assertLayout(page, width, height, true);
      await page.screenshot({ path: `/tmp/clash-recap-dock-stress-fr-${width}x${height}.png` });
      // Resize the same recap across the breakpoint, then return without resetting it.
      if (!fixtureOnly) {
        const other = width >= 1100 ? { width: 390, height: 844 } : { width: 1400, height: 850 };
        await page.setViewportSize(other);
        await assertLayout(page, other.width, other.height, true);
        await page.setViewportSize({ width, height });
        await assertLayout(page, width, height, true);
        await page.evaluate(() => {
          window.transportNode = document.querySelector('.playback-controls');
        });
        await back.click();
        assert.equal(
          await bar.locator('.playback-content').evaluate((el) => el.scrollTop),
          0,
          'Back resets scroll to the action heading',
        );
        await next.click();
        await battle.waitFor();
        assert.equal(
          await bar.locator('.playback-content').evaluate((el) => el.scrollTop),
          0,
          'Next resets scroll to the action heading',
        );
        assert(
          await page.evaluate(() => transportNode === document.querySelector('.playback-controls')),
          'transport controls remain mounted while content changes',
        );
      }
      await bar.locator('.playback-exit').click();
      await bar.waitFor({ state: 'hidden' });
      // Full replay event explanations remain root overlays, not board-relative
      // recap content, including after crossing the responsive breakpoint.
      await page.evaluate(() => {
        host.emit('replay:start');
        host.emit('replay:to', fixture.battleCursor);
      });
      const eventOverlay = page.locator('.card-reveal:has(.event-explanation)');
      await eventOverlay.waitFor();
      assert.equal(await bar.count(), 0, 'full replay events do not create recap controls');
      assert(
        await eventOverlay.evaluate((el) => !el.closest('main.play-layout')),
        'full replay event overlay stays outside the board layout',
      );
      await page.setViewportSize({ width: width >= 1100 ? 390 : 1400, height: 850 });
      assert(await eventOverlay.isVisible(), 'full replay event overlay survives resizing');
      assert.equal(await bar.count(), 0, 'resizing event replay does not create recap dock');
      await page.evaluate(() => host.emit('replay:end'));
      await eventOverlay.waitFor({ state: 'hidden' });
      assert.deepEqual(await page.evaluate(() => sent), [], 'recap never submits a game move');
      assert.deepEqual(errors, [], 'no browser exceptions');
      console.log(
        `${width}x${height}: ${fixtureOnly ? 'fixture' : 'dock/bottom layout'}, battle dice/dismiss, Back/Next/Play/Pause/Replay/Skip, recap/full-replay resize isolation, FR event overflow verified.`,
      );
      await page.close();
    }
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.includes('--serve')) {
    const portIndex = process.argv.indexOf('--port');
    const { url } = await startPreview(portIndex >= 0 ? Number(process.argv[portIndex + 1]) : 8673);
    console.log(`Recap layout preview ready: ${url}`);
    console.log(
      'Synthetic repository battle; real Back/Next/Play/Skip. Add ?stress for overflow or ?locale=fr for French.',
    );
  } else await smoke();
}
