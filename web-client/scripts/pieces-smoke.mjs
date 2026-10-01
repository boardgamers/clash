import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { build } from 'esbuild';
import { chromium } from 'playwright';

// An isolated display case and real-game fixture; never reads or changes preview saves.
const root = new URL('..', import.meta.url).pathname;
const gallery = await build({
  stdin: {
    contents: `
    import * as THREE from 'three';
    import { PieceModels } from './src/piece-models';
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#b1c7c0');
    scene.add(new THREE.HemisphereLight('#fff4d8','#386775',2.2));
    const sun = new THREE.DirectionalLight('#ffe0a7',3.2); sun.position.set(-9,18,8); scene.add(sun);
    const renderer = new THREE.WebGLRenderer({antialias:true});
    renderer.setSize(1400,850); renderer.setPixelRatio(1);
    renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.05;
    document.body.append(renderer.domElement);
    const models = new PieceModels(c => new THREE.MeshStandardMaterial({color:c,flatShading:true}), (g,m) => new THREE.Mesh(g,m));
    const kinds = ['settlement','academy','market','temple','fortress','observatory','obelisk','port','Settler','Infantry','Cavalry','Elephant','Leader','Ship','Pirate'];
    const camera = new THREE.PerspectiveCamera(36,1400/850,0.1,120);
    camera.position.set(3.8,10,12.8); camera.lookAt(0,0,0);
    const items=[];
    kinds.forEach((kind,i)=>{
      const model = i === 0 ? models.settlement('#5086af') : i < 8 ? models.building(kind,'#5086af') : models.unit(kind === 'Leader' ? {Leader:'SunTzu'} : kind === 'Pirate' ? 'Ship' : kind,'#5086af',kind === 'Pirate');
      const x=(i%5-2)*2.1, z=(Math.floor(i/5)-1)*2.5;
      model.position.set(x,0,z); scene.add(model);
      const plinth = new THREE.Mesh(new THREE.CylinderGeometry(0.8,0.82,0.1,6),new THREE.MeshStandardMaterial({color:i>=13?'#438d95':'#9aa36b'}));
      plinth.position.set(x,-0.055,z);scene.add(plinth);
      const label=document.createElement('span'); label.textContent=kind;document.body.append(label);items.push({label,x,z});
    });
    function draw(overhead=false){
      camera.position.set(...(overhead?[0,15,0.01]:[3.8,10,12.8]));camera.lookAt(0,0,0);camera.updateMatrixWorld();
      renderer.render(scene,camera);
      for(const {label,x,z} of items){const p=new THREE.Vector3(x,0,z+0.9).project(camera);label.style.left=(p.x+1)*700+'px';label.style.top=(1-p.y)*425+'px';}
    }
    window.overhead=()=>draw(true);draw();window.ready=true;
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
let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'pieces-ui', {});
for (const civ of ['China', 'Rome']) {
  state = typeof state === 'string' ? state : JSON.stringify(state);
  state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(state));
}
const game = JSON.parse(state),
  me = game.players[0],
  other = game.players[1];
for (const t of game.map.tiles) t[1] = ['Fertile', 'Forest', 'Mountain', 'Barren'][t[0].charCodeAt(0) % 4];
me.cities = [
  { position: 'C3', mood_state: 'Happy', city_pieces: { academy: 0, market: 0, observatory: 0, temple: 0 } },
  { position: 'E3', mood_state: 'Neutral', city_pieces: { fortress: 0, obelisk: 0, port: 0 } },
];
other.cities = [{ position: 'D6', mood_state: 'Angry', city_pieces: { temple: 1, market: 0 } }];
me.units = [
  ...['Settler', 'Infantry', 'Cavalry', 'Elephant'].map((unit_type, id) => ({
    id,
    unit_type,
    position: 'C3',
  })),
  { id: 4, unit_type: { Leader: 'SunTzu' }, position: 'E3' },
  { id: 5, unit_type: 'Infantry', position: 'D4' },
  { id: 6, unit_type: 'Settler', position: 'D4' },
  { id: 7, unit_type: 'Cavalry', position: 'D5' },
  { id: 8, unit_type: 'Elephant', position: 'E5' },
  { id: 9, unit_type: 'Ship', position: 'C5' },
];
me.next_unit_id = 10;
other.units = [{ id: 0, unit_type: 'Infantry', position: 'D6' }];
game.map.tiles.find((t) => t[0] === 'C5')[1] = 'Water';
game.map.tiles.find((t) => t[0] === 'D4')[1] = 'Mountain';
game.map.tiles.find((t) => t[0] === 'D5')[1] = 'Forest';
const server = createServer(async (req, res) => {
  if (req.url === '/gallery')
    res.end(
      '<!doctype html><style>body{margin:0}span{position:absolute;transform:translateX(-50%);font:14px system-ui;color:#24352f}</style><body><script src="/gallery.js"></script>',
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
  let page = await browser.newPage({ viewport: { width: 1400, height: 850 } });
  page.on('pageerror', (e) => console.error('Gallery error:', e));
  page.on('console', (m) => {
    if (m.type() === 'error') console.error(m.text());
  });
  await page.goto('http://127.0.0.1:' + server.address().port + '/gallery');
  await page.waitForFunction(() => window.ready);
  await page.screenshot({ path: '/tmp/clash-pieces-gallery.png' });
  await page.evaluate(() => window.overhead());
  await page.screenshot({ path: '/tmp/clash-pieces-overhead.png' });
  await page.close();
  for (const [width, dark] of [
    [1400, false],
    [390, true],
  ]) {
    const errors = [];
    page = await browser.newPage({ viewport: { width, height: 900 } });
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto('http://127.0.0.1:' + server.address().port);
    await page.evaluate(
      ({ state, dark }) => {
        window.host = clash3d.launch('#app');
        window.ready = false;
        host.on('ready', () => (window.ready = true));
        host.emit('player', { index: 0 });
        host.emit('preferences', { sound: false });
        host.emit('theme', { dark });
        host.emit('state', state);
      },
      { state: engine.stripSecret(JSON.stringify(game), 0), dark },
    );
    await page.waitForFunction(() => window.ready);
    // Same zoom available through the real controls, no forced camera in the product.
    await page
      .getByRole('button', { name: 'Zoom in', exact: true })
      .click({ clickCount: width < 500 ? 2 : 1 });
    await page.waitForTimeout(500);
    await page.screenshot({ path: '/tmp/clash-pieces-board-' + width + '.png' });
    assert.equal(await page.locator('.city-map-label').count(), 3);
    if (width === 1400) {
      // Pick the visible infantry mesh on the mountain, then inspect a building.
      await page.mouse.click(692, 450);
      await page.getByRole('region', { name: 'Unit movement', exact: true }).waitFor();
      assert.equal(
        (await page.locator('.unit-choice[aria-pressed="true"] strong').textContent()).trim(),
        'Infantry #6',
      );
      await page.getByRole('button', { name: 'Close movement controls', exact: true }).click();
      await page.locator('.city-map-label').first().click();
      await page.getByRole('region', { name: 'Tile C3 actions', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Close tile actions', exact: true }).click();
    }
    await page.getByRole('button', { name: 'Toggle top-down view', exact: true }).click();
    await page.waitForTimeout(350);
    await page.screenshot({ path: '/tmp/clash-pieces-board-overhead-' + width + '.png' });
    assert.deepEqual(errors, []);
    console.log('Models render without browser errors at', width);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
