import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
let initial = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'availability-ui', {});
for (const civilization of ['Carthage', 'Maya'])
  initial = engine.tryMove(
    initial,
    JSON.stringify({ ChooseCivilization: civilization }),
    engine.currentPlayer(initial),
  );
const seat = engine.currentPlayer(initial);
initial = JSON.parse(initial);
const me = initial.players[seat];
me.advances = ['Farming', 'Mining', 'Writing', 'Tactics', 'Myths'];
me.resources = { food: 4, wood: 3, ore: 3 };
const occupied = initial.players.flatMap((p) => p.cities?.map((c) => c.position) ?? []);
const forest = initial.map.tiles.find(([pos, terrain]) => terrain === 'Forest' && !occupied.includes(pos))[0];
const second = initial.map.tiles.find(
  ([pos, terrain]) => terrain === 'Mountain' && !occupied.includes(pos),
)[0];
me.cities.push({ ...structuredClone(me.cities[0]), position: second });
const city = me.cities[0].position;
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390, 320]) {
    let state = structuredClone(initial),
      sent = [];
    const page = await browser.newPage({ viewport: { width, height: width === 1400 ? 900 : 780 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    const emit = async () =>
      page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(JSON.stringify(state), seat));
    await page.exposeFunction('applyMove', async (action) => {
      sent.push(JSON.parse(action));
      state = JSON.parse(engine.tryMove(JSON.stringify(state), action, seat));
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
    await page.evaluate((seat) => {
      window.host = clash3d.launch('#app');
      window.prefs = { sound: false, unitBadges: true };
      host.on('update:preference', ({ name, value }) => {
        prefs = { ...prefs, [name]: value };
        host.emit('preferences', prefs);
      });
      host.on('move', (action) => window.applyMove(action));
      host.emit('player', { index: seat });
      host.emit('theme', { dark: true });
      host.emit('preferences', prefs);
    }, seat);
    await emit();
    const filter = page.getByRole('checkbox', { name: 'Available only', exact: true });
    await page.getByRole('button', { name: 'Research tree', exact: true }).click();
    const researchFilter = page.getByRole('combobox', { name: 'Filter advances', exact: true });
    await researchFilter.waitFor();
    assert.equal(await researchFilter.inputValue(), 'available');
    const v = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(state), seat), seat));
    assert.equal(await page.locator('.research-node').count(), v.advances.filter((a) => a.action).length);
    assert.equal(await page.locator('#research-Writing').count(), 0, 'researched advances are hidden');
    await researchFilter.selectOption('owned');
    assert.equal(await page.locator('.research-node').count(), v.advances.filter((a) => a.owned).length);
    assert.equal(await page.locator('.research-node:not(.owned)').count(), 0);
    await researchFilter.selectOption('all');
    assert.equal(await page.locator('.research-node').count(), v.advances.length);
    const categories = page.getByRole('navigation', { name: 'Research categories', exact: true });
    for (const [government, prerequisite, id, leading] of [
      ['Democracy', 'Philosophy', 'Philosophy', 'Voting'],
      ['Autocracy', 'Draft', 'Draft', 'Nationalism'],
      ['Theocracy', 'State Religion', 'StateReligion', 'Dogma'],
    ]) {
      await categories.getByRole('button', { name: government, exact: true }).click();
      const branch = page.getByRole('region', { name: government, exact: true });
      await page.waitForFunction(() => document.querySelector('.research-tree').scrollTop === 0);
      assert.equal(await branch.locator('.research-branch-prerequisite, .research-prerequisite').count(), 0);
      const lead = page.locator(`#research-${leading}`);
      assert.match(
        (await lead.locator('.research-availability').innerText()).replace(/\s+/g, ' '),
        new RegExp(`Needs ${prerequisite}`),
      );
      if (government === 'Theocracy')
        await page.screenshot({ path: `/tmp/clash-government-prerequisite-${width}.png` });
      await lead.getByRole('button', { name: prerequisite, exact: true }).click();
      assert(await page.locator(`#research-${id}.selected`).isVisible());
      assert.equal(await page.locator('.research-detail h3').innerText(), prerequisite);
      const unlock = page.locator(`#research-${id} .research-effects`);
      assert.match((await unlock.innerText()).replace(/\s+/g, ' '), new RegExp(`Unlocks ${leading}`));
      if (government === 'Democracy') {
        await page.locator(`#research-${id}`).scrollIntoViewIfNeeded();
        await page.screenshot({ path: `/tmp/clash-research-unlocks-${width}.png` });
      }
      await unlock.getByRole('button', { name: leading, exact: true }).click();
      assert(await page.locator(`#research-${leading}.selected`).isVisible());
    }
    await researchFilter.selectOption('available');
    await categories.getByRole('button', { name: 'Education', exact: true }).click();
    assert.equal(await page.locator('#research-Voting').count(), 0);
    await page.locator('#research-Philosophy').getByRole('button', { name: 'Voting', exact: true }).click();
    assert(
      await page.locator('#research-Voting.selected').isVisible(),
      'forward links reveal government advances hidden by the Available filter',
    );
    assert.equal(await researchFilter.inputValue(), 'all');
    assert.deepEqual(sent, [], 'prerequisite links only inspect research');
    await researchFilter.selectOption('available');
    await categories.getByRole('button', { name: 'All advances', exact: true }).click();
    await page.screenshot({ path: `/tmp/clash-available-research-${width}.png` });
    await page.getByRole('button', { name: 'Close research', exact: true }).click();
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    assert(await filter.isChecked());
    const offers = v.cityActions.find((c) => c.position === city);
    assert.equal(
      await page.locator('.building-option').count(),
      offers.buildings.filter((b) => b.choices.length && !b.owned).length,
    );
    assert((await page.locator('.building-option').count()) > 0);
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    assert.equal(await page.getByRole('article', { name: 'Elephant', exact: true }).count(), 0);
    assert.equal(await page.locator('.leader-card').count(), 0, 'unaffordable leaders are hidden');
    await page.getByRole('button', { name: 'Add Infantry', exact: true }).click();
    await filter.uncheck();
    assert.equal(await page.getByRole('article', { name: 'Elephant', exact: true }).count(), 1);
    assert(await page.getByRole('button', { name: 'Add Elephant', exact: true }).isDisabled());
    assert.equal(await page.locator('.leader-card').count(), 3);
    await filter.check();
    assert.equal(await page.getByLabel('Infantry selected', { exact: true }).innerText(), '1');
    await page.screenshot({ path: `/tmp/clash-available-recruit-${width}.png` });
    assert(await page.locator('.city-dialog').evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    const filterBox = await filter.boundingBox();
    assert(filterBox.x >= 0 && filterBox.x + filterBox.width < width, 'filter visible on mobile');
    await page.getByRole('button', { name: 'Close city management', exact: true }).click();
    state.actions_left = 0;
    state.players[seat].units[0].position = forest;
    const guard = { id: state.players[seat].next_unit_id++, position: city, unit_type: 'Infantry' };
    state.players[seat].units.unshift(guard);
    await emit();
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    await page.getByRole('button', { name: 'Buildings', exact: true }).click();
    await page.getByText('No buildings available in this city.', { exact: false }).waitFor();
    await page.getByRole('button', { name: 'Show all', exact: true }).click();
    assert.equal(await page.locator('.building-option').count(), offers.buildings.length);
    await page.getByRole('button', { name: 'Close city management', exact: true }).click();
    await page.getByRole('button', { name: 'Move units and found cities', exact: true }).click();
    const movement = page.getByRole('region', { name: 'Unit movement', exact: true });
    await movement.getByRole('button', { name: 'Show next group of units', exact: true }).click();
    assert.equal(await movement.locator('.unit-choice[aria-pressed=true]').count(), 0);
    const found = page.getByRole('button', { name: 'Found city here', exact: false });
    await found.waitFor();
    assert(await found.isDisabled());
    await page.getByText('No actions left · Founding a city needs 1 action.', { exact: true }).waitFor();
    await page.screenshot({ path: `/tmp/clash-found-forest-${width}.png` });
    assert.deepEqual(sent, []);
    await movement.getByRole('button', { name: 'Show next group of units', exact: true }).click();
    assert.equal(await found.count(), 0, 'founding does not follow the previous group');
    await movement.getByRole('button', { name: 'Show next group of units', exact: true }).click();
    state.actions_left = 1;
    await emit();
    await found.waitFor();
    assert(await found.isEnabled());
    await found.click();
    await page.waitForFunction(() => !document.querySelector('.found-city-action'));
    assert.equal(sent.length, 1);
    assert.equal(state.actions_left, 0);
    assert(state.players[seat].cities.some((c) => c.position === forest));
    // Browsing disabled leaders should not repeat the toolbar's turn/decision notice.
    state.current_player_index = 1 - seat;
    await emit();
    await page.getByRole('button', { name: 'Manage cities', exact: true }).click();
    await page.getByRole('button', { name: 'Recruit', exact: true }).click();
    if (await filter.isChecked()) await filter.uncheck();
    assert.equal(await page.locator('.leader-card').count(), 3);
    assert.equal(await page.locator('.leader-card .reason').count(), 0);
    assert(await page.locator('.leader-select').first().isDisabled());
    await page.getByRole('button', { name: 'Close city management', exact: true }).click();
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: default filters, Show all, retained selections, empty states and forest founding with/without an action verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
}
