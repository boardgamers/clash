import assert from 'node:assert/strict';
import { chromium } from 'playwright';
const base = process.env.CLASH_PREVIEW_URL ?? 'http://localhost:8679';
assert.ok(['localhost', '127.0.0.1'].includes(new URL(base).hostname), 'local preview only');
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const state = async () => {
  const data = await (await fetch(`${base}/api/state?seat=0`)).json();
  assert.equal(data.activeScenario, 'tactics');
  return JSON.parse(data.state);
};
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  await page.goto(base);
  await page.locator('#scenario option[value="tactics"]').waitFor({ state: 'attached' });
  await page.locator('#scenario').selectOption('tactics');
  const steel = page.frameLocator('iframe').getByRole('region', { name: 'Steel Weapons', exact: true });
  await steel.waitFor();
  const initial = await state();
  for (const theme of ['light', 'dark']) {
    if (theme === 'dark') await page.locator('#theme').click();
    for (const selector of ['#seat', '#scenario', '#seat option', '#scenario option']) {
      const styles = await page
        .locator(selector)
        .first()
        .evaluate((el) => {
          const s = getComputedStyle(el);
          return [s.color, s.backgroundColor, s.colorScheme, s.opacity];
        });
      assert.deepEqual(styles, ['rgb(228, 232, 217)', 'rgb(35, 61, 53)', 'dark', '1']);
    }
    await page.screenshot({ path: `/tmp/clash-local-host-${theme}.png` });
  }
  await steel.getByRole('button', { name: 'Gold', exact: false }).click();
  await steel.getByRole('button', { name: 'Confirm', exact: true }).click();
  await steel.waitFor({ state: 'hidden' });
  assert.equal((await state()).players[0].resources.gold, initial.players[0].resources.gold - 1);
  const game = page.frameLocator('iframe');
  const tactics = game.getByRole('region', { name: 'Tactics', exact: true });
  await tactics.getByRole('button', { name: 'Select Routing', exact: true }).click();
  await tactics.getByRole('button', { name: 'Queue tactics', exact: true }).click();
  await page.waitForTimeout(300);
  assert.equal((await state()).early_choices.length, 1);
  await page.locator('#restart-scenario').click();
  await steel.waitFor();
  const restarted = await state();
  for (const key of ['players', 'events', 'early_choices', 'actions_left'])
    assert.deepEqual(restarted[key], initial[key], key);
  assert.equal(restarted.early_choices?.length ?? 0, 0);
  const fresh = await browser.newPage();
  await fresh.goto(base);
  await fresh.waitForFunction(
    () =>
      document.querySelector('#scenario')?.value === 'tactics' &&
      !document.querySelector('#restart-scenario')?.disabled,
  );
  console.log(
    'Host select contrast in both themes; Steel fixture selection, payment/restart and fresh-tab restart passed.',
  );
} finally {
  await browser.close();
}
