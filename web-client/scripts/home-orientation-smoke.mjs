// Run after npm run build. Exercise the real BGS viewer callbacks and rendered camera,
// not a second implementation of the camera math.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { hostPreferences } from './host-preferences-fixture.mjs';
const engine = createRequire(import.meta.url)('../.engine/server.js');
// Optional anonymous /api/gameplay response, kept outside the repository:
// node scripts/home-orientation-smoke.mjs --snapshot=/tmp/public-game.json --seat=1
// Never strip it again: retain public setup log and board history for home resolution.
const snapshotPath = process.argv.find((arg) => arg.startsWith('--snapshot='))?.slice(11);
const seatArg = process.argv.find((arg) => arg.startsWith('--seat='))?.slice(7);
const selectedSeat = seatArg === undefined ? undefined : Number(seatArg);
if (selectedSeat !== undefined) assert(Number.isInteger(selectedSeat) && selectedSeat >= 0);
const snapshot = snapshotPath ? JSON.parse(await readFile(snapshotPath, 'utf8')) : undefined;
const snapshotRaw = snapshot
  ? typeof snapshot.data === 'string'
    ? snapshot.data
    : JSON.stringify(snapshot)
  : undefined;
const snapshotGame = snapshotRaw ? JSON.parse(snapshotRaw) : undefined;
if (snapshotGame)
  assert.equal(snapshotGame.secrets_stripped, true, 'Only use a public, secrets-stripped snapshot');
const counts = snapshotGame
  ? [snapshotGame.players.filter((p) => !['Barbarians', 'Pirates'].includes(p.civilization)).length]
  : [2, 3, 4];
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
let checks = 0;
try {
  for (const count of counts) {
    if (selectedSeat !== undefined) assert(selectedSeat < count, 'Seat must be a human player index');
    const raw = snapshotRaw ?? (await engine.init(count, [], {}, 'home-orientation', {}));
    const game = JSON.parse(raw);
    const homes = game.players.slice(0, count).map((p) => p.cities[0].position);
    for (const mapView of ['3d', 'strategy']) {
      const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('console', (message) => {
        if (
          message.type() === 'warning' &&
          /event: preferences|invalid_format|Invalid payload/.test(message.text())
        )
          errors.push(message.text());
      });
      await page.route('**/*', async (route) => {
        const p = new URL(route.request().url()).pathname;
        if (p === '/')
          return route.fulfill({
            contentType: 'text/html',
            body: '<!doctype html><meta charset="utf-8"><div id="app"></div><script src="/viewer.js"></script>',
          });
        if (!['/viewer.js', '/server_bg.wasm'].includes(p)) return route.abort();
        return route.fulfill({
          contentType: p.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
          body: await readFile(new URL('../dist' + p, import.meta.url)),
        });
      });
      const settle = () =>
        page.evaluate(
          () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
        );
      const positions = async () => {
        await settle();
        await page.waitForTimeout(350); // Let strategy-label CSS transforms finish.
        return page.evaluate(
          ({ homes, mapView }) =>
            homes.map((home) => {
              const node =
                mapView === 'strategy'
                  ? document.querySelector(`.strategy-map-tile[data-position="${home}"]`)
                  : [...document.querySelectorAll('.city-map-label')].find((n) =>
                      n.getAttribute('aria-label').includes(`city ${home} ·`),
                    );
              if (!node) throw Error(`Missing home ${home}`);
              const box = node.getBoundingClientRect();
              return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
            }),
          { homes, mapView },
        );
      };
      const bottom = async (seat, stage) => {
        const points = await positions();
        assert(
          points.every((p, i) => i === seat || points[seat].y > p.y + 5),
          `${count}p ${mapView} seat ${seat} ${stage}: ${JSON.stringify(points)}`,
        );
        assert.deepEqual(errors, []);
        checks++;
      };
      for (let seat = 0; seat < count; seat++) {
        if (selectedSeat !== undefined && seat !== selectedSeat) continue;
        for (const order of ['player-first', 'state-first', 'preferences-last']) {
          await page.goto('http://clash.test/');
          const publicState = snapshotRaw ?? engine.stripSecret(raw, seat);
          await page.evaluate(
            ({ publicState, seat, preferences, order }) => {
              window.host = clash3d.launch('#app');
              // Reproduce the production failure through the real registered emitter,
              // then deliver the fixed platform snapshot with HTTP(S) badge assets.
              const broken = structuredClone(preferences);
              broken.bgs.players[0].badges[0].url = 'data:image/svg+xml,%3Csvg%3E%3C/svg%3E';
              const warn = console.warn;
              const diagnostics = [];
              console.warn = (diagnostic) => diagnostics.push(diagnostic);
              let accepted;
              try {
                accepted = host.emit('preferences', broken);
              } finally {
                console.warn = warn;
              }
              if (accepted || diagnostics.length !== 1 || diagnostics[0].event !== 'preferences')
                throw Error('Expected inlined badge to reject the full preferences event');
              const emit = host.emit.bind(host);
              host.emit = (event, payload) => {
                const result = emit(event, payload);
                if (event === 'preferences' && !result) throw Error('Valid host preferences rejected');
                return result;
              };
              window.prefs = preferences;
              if (order !== 'preferences-last') host.emit('preferences', prefs);
              if (order === 'player-first') host.emit('player', { index: seat });
              host.emit('state', publicState);
            },
            { publicState, seat, preferences: hostPreferences(mapView, count), order },
          );
          await page
            .locator(
              order === 'preferences-last'
                ? '.city-map-label'
                : mapView === 'strategy'
                  ? '.strategy-map-tile'
                  : '.city-map-label',
            )
            .first()
            .waitFor();
          await settle();
          if (order !== 'player-first')
            await page.evaluate((seat) => host.emit('player', { index: seat }), seat);
          if (order === 'preferences-last') await page.evaluate(() => host.emit('preferences', prefs));
          await page
            .locator(mapView === 'strategy' ? '.strategy-map-tile' : '.city-map-label')
            .first()
            .waitFor();
          await bottom(seat, order);
          await page.evaluate((publicState) => host.emit('state', publicState + ' '), publicState);
          await bottom(seat, 'state refresh');
          await page.evaluate(() => {
            prefs.homeAtBottom = false;
            host.emit('preferences', prefs);
          });
          const normal = await positions();
          await page.evaluate(() => {
            prefs.homeAtBottom = true;
            host.emit('preferences', prefs);
          });
          await bottom(seat, 'enable after disable');
          await page.evaluate(() => {
            prefs.homeAtBottom = false;
            host.emit('preferences', prefs);
          });
          assert.deepEqual(await positions(), normal, 'disabling restores the standard camera');
          await page.evaluate(() => {
            prefs.homeAtBottom = true;
            host.emit('preferences', prefs);
          });
        }
        await page.setViewportSize({ width: 1000, height: 700 });
        await bottom(seat, 'resize');
        await page.getByRole('button', { name: 'Enter fullscreen', exact: true }).click();
        await page.waitForFunction(() => !!document.fullscreenElement);
        await bottom(seat, 'fullscreen');
        await page.evaluate(() => document.exitFullscreen());
        await page.setViewportSize({ width: 1400, height: 900 });
        await bottom(seat, 'exit fullscreen');
        const homeLabel =
          mapView === 'strategy'
            ? page.locator(`.strategy-map-tile[data-position="${homes[seat]}"]`)
            : page.locator(`.city-map-label[aria-label*="city ${homes[seat]} ·"]`);
        await homeLabel.click();
        await bottom(seat, 'city side panel');
        const closePanel = page.getByRole('button', { name: 'Close tile actions', exact: true });
        if (await closePanel.isVisible()) await closePanel.click();
        if (mapView === '3d') {
          const canvas = page.locator('canvas[aria-label*="civilization map"]');
          const box = await canvas.boundingBox();
          await page.mouse.move(box.x + box.width / 2, box.y + 40);
          await page.mouse.down();
          await page.mouse.move(box.x + box.width / 2 + 160, box.y + 40, { steps: 8 });
          await page.mouse.up();
          const rotated = await positions();
          await page.evaluate(() => host.emit('preferences', prefs));
          assert.deepEqual(await positions(), rotated, 'preference snapshot must not lock the user camera');
          await canvas.focus();
          await page.keyboard.press('Home');
          await bottom(seat, 'explicit camera reset after orbit');
        }
      }
      await page.close();
      console.log(
        `${count}p ${mapView}: ${selectedSeat === undefined ? 'every seat' : `seat ${selectedSeat}`} and event order passed`,
      );
    }
  }
  console.log(`${checks} rendered home-bottom assertions passed`);
} finally {
  await browser.close();
}
