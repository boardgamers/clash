import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

export async function influenceFixture(engine) {
  let raw = await engine.init(3, [], { civilization: 'ChooseCivilization' }, 'influence-preview', {});
  for (const civ of ['Rome', 'Greece', 'Egypt']) {
    raw = engine.tryMove(raw, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(raw));
  }
  const game = JSON.parse(raw);
  const cols = ['C', 'D', 'E', 'F', 'G'];
  const terrainAt = (col, row) => {
    if (col === 'G') return 'Water';
    if (col === 'D' && row === 2) return 'Mountain';
    if ((col === 'C' && row === 5) || (col === 'E' && row === 3) || (col === 'D' && row === 6)) return 'Forest';
    if (col === 'E' && row === 6) return 'Barren';
    return 'Fertile';
  };
  game.map.tiles = [];
  for (const col of cols) {
    for (let row = 2; row <= 6; row++) {
      game.map.tiles.push([col + row, terrainAt(col, row)]);
    }
  }
  if (game.map.unexplored_blocks) game.map.unexplored_blocks = [];
  for (const player of game.players) {
    player.cities = [];
    player.units = [];
  }
  game.players[0].civilization = 'Rome';
  game.players[1].civilization = 'Greece';
  game.players[2].civilization = 'Egypt';
  // 4 nearby Roman cities demonstrating control, single-civ, counted single-civ, and multi-civ influence.
  game.players[0].cities = [
    {
      position: 'C3',
      mood_state: 'Happy',
      city_pieces: { temple: 0, fortress: 0 },
    },
    {
      position: 'F3',
      mood_state: 'Happy',
      city_pieces: { temple: 1 },
    },
    {
      position: 'F5',
      mood_state: 'Neutral',
      city_pieces: { academy: 1, port: 1, temple: 1, fortress: 0 },
      port_position: 'G5',
    },
    {
      position: 'C5',
      mood_state: 'Angry',
      city_pieces: { temple: 1, obelisk: 2 },
    },
  ];
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

export async function startPreview(port = 8646) {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const game = await influenceFixture(engine);
  const raw = JSON.stringify(game);
  const state = engine.stripSecret(raw, 0);
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://localhost');
    const path = url.pathname;
    if (path === '/') {
      const dark = url.searchParams.get('dark') === '1';
      const colorBlind = url.searchParams.get('colorblind') === '1';
      const mapView = url.searchParams.get('strategy') === '1' ? 'strategy' : '3d';
      const locale = url.searchParams.get('locale') || 'en';
      res.setHeader('Content-Type', 'text/html');
      return res.end(`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Clash · Cultural influence preview</title><div id="app"></div><script src="/viewer.js"></script><script>
        window.host = clash3d.launch('#app');
        window.sent = [];
        window.prefs = { sound: false, mapView: ${JSON.stringify(mapView)}, colorBlind: ${JSON.stringify(colorBlind)}, locale: ${JSON.stringify(locale)} };
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
  console.log('Cultural influence preview: http://localhost:8646/ (normal viewer, 3D + Strategy)');
}
