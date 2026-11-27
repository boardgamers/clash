import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

function rollLogAction(text) {
  return {
    player: 0,
    action: 'StartTurn',
    items: [
      {
        player: 0,
        origin: { Ability: 'Roll to End' },
        Text: text,
      },
    ],
  };
}

export async function rollToEndFixture(engine, { ended = false } = {}) {
  const raw = await engine.init(2, [], { length: 'RollToEnd' }, 'roll-to-end-preview', {});
  const game = JSON.parse(raw);

  const age6 = {
    age: 6,
    rounds: [
      {
        round: 2,
        turns: [
          {
            turn_type: { Player: 0 },
            actions: [rollLogAction('Roll to End: 2 (end on 6)')],
          },
        ],
      },
      {
        round: 3,
        turns: [
          {
            turn_type: { Player: 0 },
            actions: [rollLogAction('Roll to End: 3 (end on 5–6)')],
          },
        ],
      },
    ],
  };

  game.log.push(age6);

  if (ended) {
    game.age = 7;
    game.round = 1;
    game.variable_end = 'RollToEnd';
    game.log.push({
      age: 7,
      rounds: [
        {
          round: 1,
          turns: [
            {
              turn_type: { Player: 0 },
              actions: [rollLogAction('Roll to End: 5 (end on 4–6)')],
            },
          ],
        },
      ],
    });
  } else {
    game.age = 6;
    game.round = 3;
  }

  game.log_index = 1;
  return game;
}

const contentType = (path) =>
  path.endsWith('.wasm')
    ? 'application/wasm'
    : path.endsWith('.json')
      ? 'application/json'
      : path.endsWith('.woff2')
        ? 'font/woff2'
        : 'text/javascript';

export async function startPreview(port = 8647) {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const activeState = engine.stripSecret(JSON.stringify(await rollToEndFixture(engine, { ended: false })), 0);
  const endedState = engine.stripSecret(JSON.stringify(await rollToEndFixture(engine, { ended: true })), 0);

  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://localhost');
    const path = url.pathname;
    if (path === '/') {
      const dark = url.searchParams.get('dark') === '1';
      const ended = url.searchParams.get('ended') === '1';
      const locale = url.searchParams.get('locale') || 'en';
      const state = ended ? endedState : activeState;
      res.setHeader('Content-Type', 'text/html');
      return res.end(`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Clash · Roll to End preview</title><div id="app"></div><script src="/viewer.js"></script><script>
        window.host = clash3d.launch('#app');
        window.sent = [];
        window.prefs = { sound: false, locale: ${JSON.stringify(locale)} };
        host.on('move', (a) => sent.push(a));
        host.on('update:preference', ({ name, value }) => { prefs[name] = value; host.emit('preferences', prefs); });
        host.emit('player', { index: 0 });
        host.emit('theme', { dark: ${JSON.stringify(dark)} });
        host.emit('preferences', prefs);
        host.emit('state', ${JSON.stringify(state)});
      </script>`);
    }
    if (!/^\/(viewer\.js|server_bg\.wasm|[a-zA-Z-]+\.json|[a-zA-Z-]+\.woff2)$/.test(path)) {
      res.statusCode = 404;
      return res.end();
    }
    try {
      res.setHeader('Content-Type', contentType(path));
      res.end(await readFile(new URL('../dist' + path, import.meta.url)));
    } catch {
      res.statusCode = 404;
      res.end();
    }
  });
  await new Promise((resolve) => server.listen(port, '127.0.0.1', resolve));
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await startPreview();
  console.log('Roll to End preview: http://localhost:8647/ (?ended=1, ?dark=1)');
}
