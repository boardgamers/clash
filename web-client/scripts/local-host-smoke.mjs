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
  const testWidth = Number(process.env.CLASH_TEST_WIDTH ?? 1400);
  const page = await browser.newPage({ viewport: { width: testWidth, height: 900 } });
  const submitted = [];
  page.on('request', (request) => {
    if (request.url().endsWith('/api/move') && request.method() === 'POST') {
      const body = request.postDataJSON();
      submitted.push(typeof body.move === 'string' ? JSON.parse(body.move) : body.move);
    }
  });
  if (process.env.VIEWER_URL) {
    await page.route('**/bundle/**', async (route) => {
      const name = new URL(route.request().url()).pathname.slice('/bundle/'.length);
      const response = await fetch(new URL(name, process.env.VIEWER_URL));
      assert(response.ok, name);
      await route.fulfill({
        contentType: name.endsWith('.wasm')
          ? 'application/wasm'
          : name.endsWith('.json')
            ? 'application/json'
            : 'text/javascript',
        body: Buffer.from(await response.arrayBuffer()),
      });
    });
  }
  await page.goto(`${base}/?built`);
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
  await tactics.waitFor();
  const ownTactics = await state();
  assert.equal(ownTactics.events.at(-1).player, 0);
  assert.deepEqual(ownTactics.map, initial.map);
  for (const width of [1400, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.screenshot({ path: `/tmp/clash-async-own-tactics-${width}.png` });
  }
  await page.setViewportSize({ width: testWidth, height: 900 });
  await tactics.getByRole('button', { name: 'Select Routing', exact: true }).click();
  await tactics.getByRole('button', { name: 'Play tactics', exact: true }).click();
  await page.waitForTimeout(300);
  const committed = await state();
  assert.equal(committed.early_choices?.length ?? 0, 0);
  assert.equal(committed.events.at(-1).player, 1);
  assert.equal(committed.events.at(-1).event_type.CombatRoundStart.preparation, 'DefenderAbilities');
  for (const width of [1400, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.screenshot({ path: `/tmp/clash-async-combat-${width}.png` });
  }
  // The attacker has committed, not queued: the other seat and spectator see
  // only a played-card marker until both ordinary Tactics responses complete.
  for (const seat of ['1', 'spectator']) {
    const hidden = JSON.parse((await (await fetch(`${base}/api/state?seat=${seat}`)).json()).state);
    assert.equal(hidden.events.at(-1).event_type.CombatRoundStart.attacker_strength.tactics_card, 0);
    assert(!hidden.action_cards_discarded?.includes(3));
    assert.equal(hidden.early_choices?.length ?? 0, 0);
  }
  await page.setViewportSize({ width: testWidth, height: 900 });
  await page.locator('#seat').selectOption('1');
  await steel.waitFor();
  await steel.getByRole('button', { name: 'Gold', exact: false }).click();
  await steel.getByRole('button', { name: 'Confirm', exact: true }).click();
  await tactics.waitFor();
  const defenderPaid = await state();
  assert.equal(defenderPaid.players[1].resources.gold, initial.players[1].resources.gold - 1);
  assert.equal(defenderPaid.events.at(-1).player, 1);
  assert.equal(await game.getByRole('button', { name: /Queue tactics/ }).count(), 0);
  await tactics.getByRole('button', { name: 'Select Peltasts', exact: true }).click();
  await tactics.getByRole('button', { name: 'Play tactics', exact: true }).click();
  await page.waitForFunction(async () => {
    const raw = JSON.parse((await (await fetch('/api/state?seat=0')).json()).state);
    return raw.log
      .flatMap((a) => a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])))
      .some((a) => a.items?.some((i) => i.CombatRoll));
  });
  const rolled = await state();
  const actions = rolled.log.flatMap((a) => a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])));
  const rolls = (items) => items.flatMap((a) => a.items ?? []).filter((i) => i.CombatRoll).length;
  const initialActions = initial.log.flatMap((a) =>
    a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])),
  );
  assert(rolls(actions) >= rolls(initialActions) + 2, 'both sides actually rolled after the commitments');
  assert.deepEqual(
    submitted,
    [
      { Response: { Payment: [{ gold: 1 }] } },
      { Response: { SelectHandCards: [{ ActionCard: 3 }] } },
      { Response: { Payment: [{ gold: 1 }] } },
      { Response: { SelectHandCards: [{ ActionCard: 4 }] } },
    ],
    'both normal payments and card responses commit exactly once; no queue action',
  );
  assert.equal(rolled.early_choices?.length ?? 0, 0);
  await page.locator('#seat').selectOption('0');
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
    `${testWidth}px: both Steel payments and ordinary Tactics commits, handoff, hidden-card privacy, both dice rolls, no combat queue, host themes and restart passed.`,
  );
} finally {
  await browser.close();
}
