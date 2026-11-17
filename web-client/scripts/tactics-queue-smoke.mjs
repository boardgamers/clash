import assert from 'node:assert/strict';
import { chromium } from 'playwright';
const base = process.env.CLASH_PREVIEW_URL ?? 'http://localhost:8679';
const browser = await chromium.launch({
  headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader'],
});
assert.ok(['localhost', '127.0.0.1'].includes(new URL(base).hostname), 'local preview only');
const state = async (seat) => await (await fetch(`${base}/api/state?seat=${seat}`)).json();
try {
  for (const [width, height] of [
    [1400, 900],
    [390, 740],
    [320, 640],
  ]) {
    await fetch(`${base}/api/scenarios`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id: 'tactics' }),
    });
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(`${base}/?built`);
    await page.locator('#seat').selectOption('0');
    const game = page.frameLocator('iframe');
    const panel = game.getByRole('region', { name: 'Steel Weapons', exact: true });
    await panel.waitFor();
    assert.equal(await panel.locator('.tactics-queue').count(), 0);
    assert.equal(await game.getByRole('region', { name: 'Choose ahead', exact: true }).count(), 0);
    await page.screenshot({ path: `/tmp/clash-tactics-steel-${width}.png` });
    await panel.getByRole('button', { name: 'Gold', exact: false }).click();
    const before = JSON.parse((await state(0)).state);
    await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
    await panel.waitFor({ state: 'hidden' });
    const paid = JSON.parse((await state(0)).state);
    assert.equal(paid.players[0].resources.gold, before.players[0].resources.gold - 1);
    assert.equal(paid.players[0].resources.ore, before.players[0].resources.ore);
    const commit = async (name) => {
      const response = page.waitForResponse(
        (r) => r.url().endsWith('/api/move') && r.request().method() === 'POST',
      );
      await choices.getByRole('button', { name, exact: true }).click();
      assert.equal((await response).status(), 200);
    };
    const choices = game.getByRole('region', { name: 'Tactics', exact: true });
    await choices.waitFor();
    assert.equal(await game.getByRole('region', { name: 'Choose ahead', exact: true }).count(), 0);
    assert(await choices.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
    await page.screenshot({ path: `/tmp/clash-tactics-queue-${width}.png` });
    await choices.getByRole('button', { name: 'Select Routing', exact: true }).click();
    assert.doesNotMatch(await choices.innerText(), /Discards the selected card/);
    await commit('Queue tactics');
    await choices.getByText('Queued', { exact: true }).waitFor();
    const queued = JSON.parse((await state(0)).state);
    for (const field of ['players', 'events', 'log', 'log_index', 'board_history', 'time_increments'])
      assert.deepEqual(queued[field], paid[field], field);
    assert.equal(queued.early_choices.length, 1);
    assert.equal(JSON.parse((await state(1)).state).early_choices, undefined);
    await choices.getByRole('button', { name: 'Select High Morale', exact: true }).click();
    assert.doesNotMatch(await choices.innerText(), /Discards the selected card/);
    await commit('Queue tactics');
    await commit('Cancel queued choice');
    await choices.getByText('Queued', { exact: true }).waitFor({ state: 'hidden' });
    await commit('Skip');
    await choices.getByText('Queued', { exact: true }).waitFor();
    assert.deepEqual(JSON.parse((await state(0)).state).early_choices[0].response, { SelectHandCards: [] });
    await choices.getByRole('button', { name: 'Select Routing', exact: true }).click();
    assert.doesNotMatch(await choices.innerText(), /Discards the selected card/);
    await commit('Queue tactics');
    await choices.getByText('Queued', { exact: true }).waitFor();
    await page.locator('#seat').selectOption('1');
    await panel.waitFor({ timeout: 10000 }).catch(async (e) => {
      console.log(await page.frames()[1].locator('body').innerText(), errors);
      throw e;
    });
    await panel.getByRole('button', { name: 'Confirm', exact: true }).click();
    const tactics = game.getByRole('region', { name: 'Tactics', exact: true });
    await tactics.waitFor();
    await tactics.getByRole('button', { name: 'Select Peltasts', exact: true }).click();
    assert.doesNotMatch(await tactics.innerText(), /Discards the selected card/);
    assert.equal(await tactics.getByRole('button', { name: 'Play tactics', exact: true }).count(), 1);
    assert.equal(await tactics.locator('.tactics-queue').count(), 0, 'current prompt is not a queued choice');
    await page.screenshot({ path: `/tmp/clash-tactics-live-${width}.png` });
    assert.deepEqual(errors, []);
    await page.close();
    console.log(
      `Steel first, automatic normal tactics queue/edit/cancel, privacy and ordinary live prompt: ${width}px passed`,
    );
  }
} finally {
  await browser.close();
}
