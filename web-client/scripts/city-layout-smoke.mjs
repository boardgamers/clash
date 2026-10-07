import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { build } from 'esbuild';
import { chromium } from 'playwright';

// Close-ups of crowded cities for the civilizations with the widest roofs and temples,
// then the same cities on the real board (requires `vite build` and `build:bridge`).
// Usage: node scripts/city-layout-smoke.mjs [output prefix]
const prefix = process.argv[2] ?? '/tmp/clash-city-layout';
const root = new URL('..', import.meta.url).pathname;
const civilizations = ['Celts', 'Japan', 'Maya', 'Egypt', 'Rome', 'Vikings'];
const cities = [
  { buildings: ['temple', 'fortress'] },
  { buildings: ['obelisk', 'temple', 'port'] },
  { buildings: ['observatory', 'temple', 'fortress', 'academy'] },
  { buildings: ['obelisk', 'temple', 'port', 'market'] },
  { buildings: ['temple', 'market'], wonders: ['GreatWall'] },
  { buildings: ['temple', 'fortress', 'port', 'academy'], wonders: ['Pyramids', 'GreatStatue'] },
];
const gallery = await build({
  stdin: {
    contents: `
    import * as THREE from 'three';
    import { PieceModels } from './src/piece-models';
    import { buildingOrder, cityLayout, cityPose, ownershipMarkers } from './src/city-layout';
    const civilizations = ${JSON.stringify(civilizations)}, cities = ${JSON.stringify(cities)};
    const cell = [320, 260], columns = cities.length;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(cell[0] * columns, cell[1] * civilizations.length);
    renderer.setPixelRatio(1);
    renderer.setScissorTest(true);
    renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.05;
    document.body.append(renderer.domElement);
    const models = new PieceModels(c => new THREE.MeshStandardMaterial({ color: c, flatShading: true }), (g, m) => new THREE.Mesh(g, m));
    civilizations.forEach((civilization, row) => cities.forEach((city, column) => {
      const scene = new THREE.Scene();
      scene.background = new THREE.Color('#b1c7c0');
      scene.add(new THREE.HemisphereLight('#fff4d8', '#386775', 2.2));
      const sun = new THREE.DirectionalLight('#ffe0a7', 3.2); sun.position.set(-9, 18, 8); scene.add(sun);
      const tile = new THREE.Mesh(new THREE.CylinderGeometry(1, 1, 0.3, 6), new THREE.MeshStandardMaterial({ color: '#9aa36b' }));
      tile.rotation.y = Math.PI / 6; tile.position.y = cityPose.y - 0.15; scene.add(tile);
      const kinds = [...city.buildings].sort((a, b) => buildingOrder.indexOf(a) - buildingOrder.indexOf(b));
      const wonders = city.wonders ?? [];
      const layout = cityLayout(kinds.length, wonders.length);
      const model = new THREE.Group();
      model.position.y = cityPose.y; model.rotation.y = cityPose.rotation;
      const put = (piece, place) => { piece.scale.setScalar(place.scale); piece.position.set(place.x, 0, place.z); model.add(piece); };
      put(models.settlement('#5086af', civilization), layout.settlement);
      kinds.forEach((kind, i) => put(models.building(kind, i % 2 ? '#b96c4c' : '#5086af', civilization), layout.buildings[i]));
      wonders.forEach((kind, i) => put(models.wonder(kind, '#5086af'), layout.wonders[i]));
      const { pole, flag } = ownershipMarkers;
      const flagPole = new THREE.Mesh(new THREE.CylinderGeometry(pole.radiusTop, pole.radiusBottom, pole.height, 6), new THREE.MeshStandardMaterial({ color: '#624d30' }));
      flagPole.position.set(pole.x + layout.ownership.x, pole.y, pole.z + layout.ownership.z);
      const banner = new THREE.Mesh(new THREE.BoxGeometry(flag.width, flag.height, flag.depth), new THREE.MeshStandardMaterial({ color: '#5086af' }));
      banner.position.set(flag.x + layout.ownership.x, flag.y, flag.z + layout.ownership.z);
      model.add(flagPole, banner);
      scene.add(model);
      const camera = new THREE.PerspectiveCamera(25, cell[0] / cell[1], 0.1, 50);
      camera.position.set(1.1, 3.1, 4.1); camera.lookAt(0, 0.55, 0);
      const x = column * cell[0], y = (civilizations.length - 1 - row) * cell[1];
      renderer.setViewport(x, y, cell[0], cell[1]); renderer.setScissor(x, y, cell[0], cell[1]);
      renderer.render(scene, camera);
      const label = document.createElement('span');
      label.textContent = civilization + ' ' + (kinds.length + 1) + (wonders.length ? ' +' + wonders.length + 'W' : '');
      label.style.left = x + 6 + 'px'; label.style.top = row * cell[1] + 4 + 'px';
      document.body.append(label);
    }));
    window.ready = true;
  `,
    resolveDir: root,
    loader: 'ts',
  },
  bundle: true,
  write: false,
  format: 'iife',
  logLevel: 'silent',
});

const engine = createRequire(import.meta.url)('../.engine/server.js');
let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'city-layout', {});
for (const civ of ['Celts', 'Japan']) {
  state = typeof state === 'string' ? state : JSON.stringify(state);
  state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(state));
}
const game = JSON.parse(state);
for (const t of game.map.tiles) t[1] = ['Fertile', 'Forest', 'Mountain', 'Barren'][t[0].charCodeAt(0) % 4];
const pieces = (buildings, owner, wonders = []) => ({
  ...Object.fromEntries(buildings.map((b) => [b, owner])),
  ...(wonders.length ? { wonders } : {}),
});
game.players[0].cities = [
  { position: 'C3', mood_state: 'Happy', city_pieces: pieces(cities[2].buildings, 0) },
  { position: 'E3', mood_state: 'Neutral', city_pieces: pieces(cities[3].buildings, 0) },
  { position: 'C5', mood_state: 'Neutral', city_pieces: pieces(cities[0].buildings, 0) },
];
game.players[1].cities = [
  { position: 'D5', mood_state: 'Happy', city_pieces: pieces(cities[2].buildings, 1) },
  { position: 'E5', mood_state: 'Neutral', city_pieces: pieces(cities[1].buildings, 1) },
];
game.players[0].units = [];
game.players[1].units = [];
const server = createServer(async (req, res) => {
  if (req.url === '/gallery')
    res.end(
      '<!doctype html><style>body{margin:0}span{position:absolute;font:600 13px system-ui;color:#24352f}</style><body><script src="/gallery.js"></script>',
    );
  else if (req.url === '/gallery.js') res.end(gallery.outputFiles[0].text);
  else if (req.url === '/')
    res.end(
      '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
    );
  else if (['/viewer.js', '/server_bg.wasm'].includes(req.url)) {
    res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
    res.end(await readFile(root + '/dist' + req.url));
  } else res.writeHead(404).end();
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  const errors = [];
  let page = await browser.newPage({
    viewport: { width: 320 * cities.length, height: 260 * civilizations.length },
  });
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('http://127.0.0.1:' + server.address().port + '/gallery');
  await page.waitForFunction(() => window.ready);
  await page.screenshot({ path: `${prefix}-gallery.png` });
  await page.close();

  page = await browser.newPage({ viewport: { width: 1400, height: 850 } });
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('http://127.0.0.1:' + server.address().port);
  await page.evaluate(
    ({ state }) => {
      window.host = clash3d.launch('#app');
      window.ready = false;
      host.on('ready', () => (window.ready = true));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false });
      host.emit('state', state);
    },
    { state: engine.stripSecret(JSON.stringify(game), 0) },
  );
  await page.waitForFunction(() => window.ready);
  await page.getByRole('button', { name: 'Zoom in', exact: true }).click({ clickCount: 5 });
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${prefix}-board.png` });
  assert.deepEqual(errors, []);
  console.log('City layouts rendered:', `${prefix}-gallery.png`, `${prefix}-board.png`);
} finally {
  await browser.close();
  server.close();
}
