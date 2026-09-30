import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
// Optional game export is read locally; all moves below run against copies.
const input = JSON.parse(
  await readFile(
    process.argv[2] ??
      new URL('../../server/tests/test_games/advances/collect_free_economy.json', import.meta.url),
    'utf8',
  ),
);
const initial = input.data ?? JSON.stringify(input);
const seat = engine.currentPlayer(initial);
const view = (state) => JSON.parse(engine.webView(state, seat));
const freeEconomy = (state) =>
  view(state).collectActions.some((a) => a.value?.Custom === 'FreeEconomyCollect');
assert(freeEconomy(initial), 'Free Economy must be available after undo');
const city = view(initial).cities.find((c) => c.canActivate && c.choices.length);
const choice = city.choices[0];
const collect = (action_type) => ({
  Playing: {
    Collect: {
      city_position: city.position,
      collections: [{ position: choice.position, pile: choice.pile, times: 1 }],
      action_type,
    },
  },
});
const advance = (state, action) => engine.tryMove(state, JSON.stringify(action), seat);
const collected = advance(initial, collect('Collect'));
assert(!freeEconomy(collected));
const undone = advance(collected, 'Undo');
assert(freeEconomy(undone));
assert.equal(engine.logLength(undone), engine.logLength(initial));
assert.equal(engine.logLength(collected), engine.logLength(initial) + 1);
assert.deepEqual(JSON.parse(undone).players[seat].resources, JSON.parse(initial).players[seat].resources);
const redone = advance(undone, 'Redo');
assert.equal(engine.logLength(redone), engine.logLength(collected));
assert.deepEqual(engine.logSlice(redone, { start: 0 }), engine.logSlice(collected, { start: 0 }));
const rewound = advance(redone, 'Undo');
let free = advance(rewound, collect({ Custom: 'FreeEconomyCollect' }));
free = advance(free, { Response: { Payment: [{ mood_tokens: 1 }] } });
assert.equal(JSON.parse(free).actions_left, JSON.parse(initial).actions_left);
assert.equal(
  JSON.parse(free).players[seat].resources.mood_tokens,
  JSON.parse(initial).players[seat].resources.mood_tokens - 1,
);
const freeGame = JSON.parse(free);
assert.equal(freeGame.log.at(-1).rounds.at(-1).turns.at(-1).actions.length, freeGame.log_index);
console.log(
  'Collect → Undo → Redo → Undo → Free Economy: valid; history rewinds, resources restore, no action spent.',
);
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/') {
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.end(
        '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
      );
    } else {
      assert(['/viewer.js', '/server_bg.wasm'].includes(req.url));
      res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
      res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
    }
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  for (const width of [1400, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      ({ state, seat }) => {
        window.host = clash3d.launch('#app');
        window.ready = false;
        host.on('ready', () => (window.ready = true));
        host.emit('player', { index: seat });
        host.emit('preferences', { sound: false });
        host.emit('theme', { dark: true });
        host.emit('state', state);
      },
      { state: engine.stripSecret(initial, seat), seat },
    );
    await page.waitForFunction(() => window.ready);
    const send = async (state) => {
      await page.evaluate((state) => window.host.emit('state', state), engine.stripSecret(state, seat));
    };
    await page.getByRole('button', { name: 'Open journal', exact: true }).click();
    const journal = page.getByRole('tabpanel', { name: 'Journal', exact: true });
    const baseline = await journal.innerText();
    await send(collected);
    await page.waitForFunction(
      (before) => document.querySelector('[aria-label="Journal"]').innerText !== before,
      baseline,
    );
    await send(undone);
    await page.waitForFunction(
      (before) => document.querySelector('[aria-label="Journal"]').innerText === before,
      baseline,
    );
    await send(redone);
    await page.waitForFunction(
      (before) => document.querySelector('[aria-label="Journal"]').innerText !== before,
      baseline,
    );
    await send(rewound);
    await page.waitForFunction(
      (before) => document.querySelector('[aria-label="Journal"]').innerText === before,
      baseline,
    );
    await page.getByRole('button', { name: 'Close table activity', exact: true }).click();
    await page.getByRole('button', { name: 'Collect resources', exact: true }).click();
    const freeButton = page.getByRole('button', { name: 'Free Economy', exact: true });
    await freeButton.click();
    assert.equal(await freeButton.getAttribute('aria-pressed'), 'true');
    await page.screenshot({ path: '/tmp/clash-undo-free-economy-' + width + '.png' });
    assert.deepEqual(errors, []);
    console.log(width + 'px: journal rewinds exactly; Free Economy selectable.');
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
