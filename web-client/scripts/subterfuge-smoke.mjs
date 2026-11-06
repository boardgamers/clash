import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const move = (state, action) => engine.tryMove(state, JSON.stringify(action), engine.currentPlayer(state));
let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'subterfuge-ui', {});
for (const civ of ['Greece', 'Japan']) state = move(state, { ChooseCivilization: civ });
const game = JSON.parse(state), player = engine.currentPlayer(state), japan = 1 - player;
game.players[player].civilization = 'Greece';
game.players[japan].civilization = 'Japan';
game.players[japan].advances.push('Tactics', 'Writing', 'Husbandry');
game.players[japan].units[0].position = game.players[player].cities[0].position;
game.players[japan].units[0].unit_type = 'Infantry';
game.players[player].action_cards = [4, 27, 11];
game.players[japan].action_cards = [9];
const beforeDeclaration = JSON.stringify(game);
const browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader'] });
try {
  for (const id of [3, 4]) for (const width of [1400, 390, 320]) {
    const fixture = JSON.parse(beforeDeclaration);
    fixture.players[player].action_cards = [id, 27, 11];
    state = move(JSON.stringify(fixture), { Playing: { ActionCard: id } });
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.route('**/*', async (route) => {
      const path = new URL(route.request().url()).pathname;
      if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>' });
      if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
      return route.fulfill({ contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', body: await readFile(new URL('../dist' + path, import.meta.url)) });
    });
    await page.goto('http://clash.test/');
    await page.evaluate(({ state, seat }) => {
      window.host = clash3d.launch('#app');
      host.emit('player', { index: seat });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    }, { state: engine.stripSecret(state, japan), seat: japan });
    const panel = page.getByRole('region', { name: 'Subterfuge', exact: true });
    await panel.waitFor();
    assert.match(await panel.innerText(), /Gain 1 advance for free/);
    assert.match(await panel.innerText(), /Normal advance prerequisites apply/);
    assert.doesNotMatch(await panel.innerText(), /Assassination|Great Ideas/);
    const spacing = await panel.locator('.announced-card').evaluate((card) => {
      const header = card.querySelector('.play-card-header').getBoundingClientRect();
      const button = card.querySelector('button').getBoundingClientRect();
      const cost = card.querySelector('.card-use-rule').getBoundingClientRect();
      const effect = card.querySelector('.announced-card-effect').getBoundingClientRect();
      return {
        sameRow: Math.abs((button.top + button.bottom) / 2 - (cost.top + cost.bottom) / 2) < 1,
        horizontalGap: cost.left - button.right,
        effectGap: effect.top - header.bottom,
      };
    });
    assert.ok(spacing.sameRow, 'Title and action cost fit on one centered row');
    assert.ok(spacing.horizontalGap >= 8, 'Title and action cost have a deliberate gap');
    assert.equal(spacing.effectGap, 8, 'Effect text has a compact gap below the header');
    await panel.getByRole('button', { name: 'Inspiration', exact: true }).click();
    const rules = page.getByRole('dialog', { name: 'Inspiration rules' });
    await rules.waitFor();
    // The printed name for the engine's Encircled tactic is Routing.
    assert.match(await rules.innerText(), id === 3 ? /Routing/ : /Peltasts/);
    assert.doesNotMatch(await rules.innerText(), id === 3 ? /Peltasts/ : /Routing/);
    await rules.getByRole('button', { name: 'Close card rules' }).click();
    assert.ok(await panel.isVisible());
    assert.ok(await panel.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []);
    console.log(`${width}px: public Inspiration ${id} rules and exact battle side visible; hidden hand absent; dialog closes back to decision.`);
    await page.close();
  }
} finally {
  await browser.close();
}
