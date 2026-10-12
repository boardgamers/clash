import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'combat-smoke', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/combat/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return engine.stripSecret(JSON.stringify(game), 0);
}
const blocked = JSON.parse(await fixture('remove_casualties_attacker.outcome'));
for (const age of blocked.log) for (const round of age.rounds) for (const turn of round.turns) for (const action of turn.actions) for (const item of action.items ?? []) {
  if (item.CombatRoll && item.player === 0) {
    item.CombatRoll.rolls.forEach(die => die.bonus = false);
    item.CombatRoll.combat_modifiers = ['High Ground prevents opponent from using dice-symbol abilities'];
  }
}
const pending = JSON.stringify(blocked);
const server = createServer(async (req, res) => {
  if (req.url === '/')
    return res.end(
      '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
    );
  if (!['/viewer.js', '/server_bg.wasm'].includes(req.url)) return res.writeHead(404).end();
  res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
  res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const [width, height, dark] of [
    [1400, 900, false],
    [390, 844, true],
    [320, 640, false],
  ]) {
    if (process.env.SMOKE_WIDTH && width !== Number(process.env.SMOKE_WIDTH)) continue;
    const page = await browser.newPage({ viewport: { width, height } }),
      errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      ({ state, dark }) => {
        window.host = clash3d.launch('#app');
        host.emit('player', { index: 0 });
        host.emit('theme', { dark });
        host.emit('preferences', { sound: false });
        host.emit('state', state);
      },
      { state: pending, dark },
    );
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    const table = page.getByRole('table', { name: 'Combat round 1', exact: true });
    await table.getByText('Dice-symbol abilities blocked by High Ground', {exact:true}).waitFor();
    assert.equal(await table.locator('.ability-blocked').count(), 2);
    assert.equal(await table.locator('.ability-blocked.activated').count(), 0);
    await page.screenshot({path:`/tmp/clash-combat-blocked-${width}.png`});
    assert.deepEqual(errors, []);
    console.log(
      `${width}px: blocked dice marker and explanation verified.`,
    );
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
