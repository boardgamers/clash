import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';

// Isolated render/picking fixture: no preview saves, API, or live game mutations.
const root = new URL('..', import.meta.url).pathname;
const engine = createRequire(import.meta.url)('../.engine/server.js');
const entry = `
import {World} from '${root}/src/board.ts';
import {Controller} from '${root}/src/controller.ts';
import {get} from 'svelte/store';
import * as THREE from 'three';
import '${root}/src/style.css';
let world, session;
window.showHarbor=(game,view)=>{
  if(world)world.destroy();
  session={...get(new Controller({},new URL(location.href)).session),game,view,seat:0};
  world=new World(document.querySelector('#app'),(position,pick)=>window.lastPick={position,pick});
  world.update(session);
};
window.zoomHarbor=()=>{for(let i=0;i<4;i++)world.zoom(.75);};
window.changeMode=(patch)=>{session={...session,...patch};world.update(session)};
window.harbor=()=>{
  world.scene.updateMatrixWorld(true); world.camera.updateMatrixWorld();
  const dock=world.pieces.find(p=>p.userData.building==='port');
  const box=new THREE.Box3().setFromObject(dock);
  const point=dock.localToWorld(new THREE.Vector3(-.08,.19,-.05)).project(world.camera);
  const rect=world.renderer.domElement.getBoundingClientRect();
  const x=rect.left+(point.x+1)*rect.width/2,y=rect.top+(1-point.y)*rect.height/2;
  return {ports:world.pieces.filter(p=>p.userData.building==='port').length,
    position:dock.position.toArray(),rotation:dock.rotation.y,data:dock.userData,
    bounds:{min:box.min.toArray(),max:box.max.toArray()},point:{x,y},hit:world.hitTarget(x,y)};
};
window.ready=true;`;
const server = await createServer({
  root,
  configFile: false,
  publicDir: false,
  plugins: [
    svelte(),
    {
      name: 'isolated-port-fixture',
      resolveId(id) {
        if (id === '/ports-smoke-entry.js') return '\0ports-entry';
      },
      load(id) {
        if (id === '\0ports-entry') return entry;
      },
      configureServer(server) {
        server.middlewares.use(async (req, res, next) => {
          if (req.url !== '/harbor' && req.url !== '/pirates') return next();
          res.setHeader('Content-Type', 'text/html');
          const script =
            req.url === '/harbor'
              ? '<script type="module" src="/ports-smoke-entry.js"></script>'
              : '<script src="/dist/viewer.js"></script>';
          res.end(
            await server.transformIndexHtml(
              req.url,
              '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><style>body{margin:0}#app{height:100vh;width:100vw}</style><div class="play-layout"><div id="app"></div></div>' +
                script,
            ),
          );
        });
      },
    },
  ],
  server: { host: '127.0.0.1', port: 0 },
});
await server.listen();
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  const page = await browser.newPage({ viewport: { width: 1000, height: 800 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(origin + '/harbor');
  await page.waitForFunction(() => window.ready);
  let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'port-shoreline', {});
  for (const civ of ['Vikings', 'Rome'])
    state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(state));
  const game = JSON.parse(state);
  game.map.tiles = [];
  for (const col of ['B', 'C', 'D', 'E', 'F'])
    for (let row = 2; row <= 6; row++) game.map.tiles.push([col + row, 'Water']);
  game.map.tiles.find(([p]) => p === 'D4')[1] = 'Fertile';
  for (const p of game.players) {
    p.cities = [];
    p.units = [];
  }
  const me = game.players[0];
  const city = { position: 'D4', mood_state: 'Happy', city_pieces: { port: 0 }, port_position: 'E4' };
  me.cities = [city];
  for (const water of ['C4', 'C5', 'D3', 'D5', 'E4', 'E5']) {
    city.port_position = water;
    me.units = [{ id: 0, unit_type: 'Ship', position: water }];
    await page.evaluate(({ game, view }) => window.showHarbor(game, view), {
      game,
      view: JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0)),
    });
    await page.waitForTimeout(200);
    const info = await page.evaluate(() => window.harbor());
    assert.equal(info.ports, 1);
    assert.equal(info.data.position, water);
    assert.equal(info.data.cityPosition, 'D4');
    assert.equal(info.position[1], 0.01, 'Wharf sits on the water surface');
    assert.equal(info.hit.position, 'D4', 'Dock opens the owning city');
    assert.equal(info.hit.pick.kind, 'city');
    await page.mouse.click(info.point.x, info.point.y);
    assert.equal((await page.evaluate(() => window.lastPick)).position, 'D4');
    await page.evaluate(() => window.changeMode({ tilePanel: true, focus: 'D4' }));
    assert.equal(
      (await page.evaluate(() => window.harbor())).hit.position,
      'D4',
      'Open inspection still opens the city',
    );
    await page.evaluate(() => window.changeMode({ tilePanel: false, seaRoutes: true }));
    assert.equal(
      (await page.evaluate(() => window.harbor())).hit.position,
      water,
      'Sea guide picks the water under the wharf',
    );
    await page.evaluate(() => window.changeMode({ seaRoutes: false, topDown: true }));
    await page.waitForTimeout(100);
    await page.screenshot({ path: `/tmp/clash-port-${water}.png` });
  }
  city.port_position = 'E4';
  me.units = Array.from({ length: 4 }, (_, id) => ({ id, unit_type: 'Ship', position: 'E4' }));
  await page.evaluate(({ game, view }) => window.showHarbor(game, view), {
    game,
    view: JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0)),
  });
  await page.waitForTimeout(200);
  await page.evaluate(() => window.zoomHarbor());
  await page.waitForTimeout(100);
  await page.screenshot({ path: '/tmp/clash-port-fleet.png' });
  await page.evaluate(() => window.changeMode({ topDown: true }));
  await page.waitForTimeout(200);
  await page.evaluate(() => window.zoomHarbor());
  await page.waitForTimeout(100);
  await page.screenshot({ path: '/tmp/clash-port-fleet-overhead.png' });
  assert.deepEqual(errors, []);
  await page.close();
  for (const width of [390, 1000]) {
    const page = await browser.newPage({ viewport: { width, height: 844 } });
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(origin + '/pirates');
    await page.evaluate(() => {
      window.host = clash3d.launch('#app');
      window.ready = false;
      host.on('ready', () => (window.ready = true));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false });
      host.emit('theme', { dark: true });
    });
    for (const phase of [1, 2, 3]) {
      const raw = await readFile(
        root + `/../server/tests/test_games/incidents/pirates_spawn.outcome${phase}.json`,
        'utf8',
      );
      await page.evaluate((raw) => host.emit('state', raw), engine.stripSecret(raw, 0));
      const panel = page.getByRole('region', {
        name: phase === 3 ? 'Pirate raid' : 'Place pirates',
        exact: true,
      });
      await panel.waitFor();
      assert.match(
        await panel.textContent(),
        phase === 3 ? /Pay 1 resource or token total/ : new RegExp(`Ship ${phase} of 2`),
      );
      if (phase < 3) assert.match(await panel.textContent(), /After placement:/);
      else assert.equal(await panel.locator('.event-raid').count(), 0);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.screenshot({ path: `/tmp/clash-pirates-${phase}-${width}.png` });
    }
    await page.close();
  }
  assert.deepEqual(errors, []);
  console.log(
    'Ports: six shores, city and water picks, fleet rendering. Pirates: separate stages on mobile and desktop.',
  );
} finally {
  await browser.close();
  await server.close();
}
