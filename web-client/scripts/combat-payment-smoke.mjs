import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'combat-payment-ui', {})).players.slice(2);
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
    for (const scenario of ['regular', 'enemy-steel', 'borrowed-steel', 'barbarian', 'decline']) {
      let state = JSON.parse(
        await readFile(
          new URL(
            '../../server/tests/test_games/combat/direct_capture_city_metallurgy.json',
            import.meta.url,
          ),
          'utf8',
        ),
      );
      state.players = [...state.players.slice(0, 2), ...structuredClone(npcs)];
      const enemySteel = scenario === 'enemy-steel' || scenario === 'borrowed-steel';
      if (!enemySteel)
        state.players[0].advances = state.players[0].advances.filter((a) => a !== 'Metallurgy');
      if (scenario === 'enemy-steel') state.players[1].advances.push('SteelWeapons');
      if (scenario === 'borrowed-steel') state.players[1].great_library_advance = 'SteelWeapons';
      if (scenario === 'barbarian') {
        state.players[2].cities = state.players[1].cities;
        state.players[2].units = [{ id: 0, position: 'C1', unit_type: 'Infantry' }];
        state.players[1].cities = [];
        state.players[1].units = [];
      }
      state = JSON.parse(
        engine.tryMove(
          JSON.stringify(state),
          JSON.stringify({ Movement: { Move: { units: [0, 1, 2, 3], destination: 'C1', payment: {} } } }),
          0,
        ),
      );
      const sent = [],
        errors = [];
      const page = await browser.newPage({ viewport: { width, height } });
      page.on('pageerror', (e) => errors.push(e.message));
      await page.exposeFunction('applyMove', async (action) => {
        sent.push(JSON.parse(action));
        state = JSON.parse(engine.tryMove(JSON.stringify(state), action, 0));
        await page.evaluate(
          (state) => {
            host.emit('state', state);
            window.moveApplied = true;
          },
          engine.stripSecret(JSON.stringify(state), 0),
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
          host.emit('player', { index: 0 });
          host.emit('theme', { dark: true });
          host.emit('preferences', { sound: false });
          host.emit('state', state);
        },
        engine.stripSecret(JSON.stringify(state), 0),
      );
      const panel = page.getByRole('region', { name: 'Steel Weapons', exact: true });
      await panel.waitFor();
      assert.match(await panel.innerText(), new RegExp(`\\+${enemySteel ? 1 : 2} combat value each round`));
      assert.equal((await panel.innerText()).includes('Enemy has Steel Weapons.'), enemySteel);
      assert.doesNotMatch(await panel.innerText(), /Use steel weapons|Pay|Decline/);
      const buttons = panel.locator('.cost-only button');
      assert.equal(await buttons.count(), 3);
      const bounds = await buttons.evaluateAll((els) =>
        els.map((el) => ({
          x: el.getBoundingClientRect().x,
          y: el.getBoundingClientRect().y,
          width: el.getBoundingClientRect().width,
          height: el.getBoundingClientRect().height,
        })),
      );
      assert(
        bounds.every((b) => b.width <= 90 && b.height >= 36),
        'Payment choices are compact touch targets',
      );
      assert(
        bounds.every((b) => Math.abs(b.y - bounds[0].y) < 1),
        'All three options fit in one row',
      );
      assert(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
      if (scenario === 'regular' || scenario === 'enemy-steel')
        await page.screenshot({ path: `/tmp/clash-steel-weapons-${scenario}-${width}.png` });
      const payment = scenario === 'decline' ? {} : scenario === 'regular' ? { gold: 1 } : { ore: 1 };
      await panel
        .getByRole('button', {
          name: scenario === 'decline' ? 'No' : scenario === 'regular' ? 'Pay 1 gold' : 'Pay 1 ore',
          exact: true,
        })
        .click();
      await page.waitForFunction(() => window.moveApplied);
      assert.deepEqual(sent, [{ Response: { Payment: [payment] } }]);
      const payments = state.log
        .flatMap((age) =>
          age.rounds.flatMap((round) =>
            round.turns.flatMap((turn) => (turn.actions ?? []).flatMap((action) => action.items ?? [])),
          ),
        )
        .filter(
          (item) =>
            item.player === 0 &&
            item.origin?.Advance === 'SteelWeapons' &&
            item.Resources?.balance === 'Loss',
        )
        .map((item) => item.Resources.resources);
      assert.deepEqual(payments, [payment]);
      assert.deepEqual(errors, []);
      await page.close();
    }
    console.log(
      `${width}px: compact Steel Weapons costs, decline, +1/+2, borrowed advance and barbarian opponents verified.`,
    );
  }
} finally {
  await browser.close();
}
