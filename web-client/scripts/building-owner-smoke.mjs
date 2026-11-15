import assert from 'node:assert/strict';
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
import {PieceModels} from '${root}/src/piece-models.ts';
import {playerColor} from '${root}/src/types.ts';
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
window.shipCount=()=>world.pieces.filter(p=>p.userData.ship).length;
window.modelProof=(kinds=['port','temple','academy'])=>{
 const models=new PieceModels(c=>new THREE.MeshStandardMaterial({color:c}),(g,m)=>new THREE.Mesh(g,m));
 const fingerprint=o=>{const result=[];o.traverse(n=>{if(n.isMesh)result.push({geometry:n.geometry.type,vertices:Array.from(n.geometry.attributes.position.array),indices:n.geometry.index?Array.from(n.geometry.index.array):null,color:n.material.color.getHexString(),position:n.position.toArray(),rotation:n.rotation.toArray(),scale:n.scale.toArray()});});return JSON.stringify(result)};
 return kinds.map(kind=>{
 const actual=kind==='port'?world.pieces.find(p=>p.userData.building==='port').children[0]:world.buildings.get('D4:building:'+kind);
 const cityModel=models.building(kind,playerColor(1,session.colorBlind,session.playerColors),'Rome');
 const normal=models.building(kind,playerColor(1,session.colorBlind,session.playerColors),'Greece');
 return {kind,name:actual.name,matchesNormalOwner:fingerprint(actual)===fingerprint(normal), differsFromCity:fingerprint(actual)!==fingerprint(cityModel)};
 });
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
          if (req.url.split('?')[0] !== '/harbor' && req.url !== '/pirates') return next();
          res.setHeader('Content-Type', 'text/html');
          const script =
            req.url.split('?')[0] === '/harbor'
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
  game.players[0].civilization = 'Rome';
  game.players[1].civilization = 'Greece';
  const me = game.players[0];
  const city = {
    position: 'D4',
    mood_state: 'Happy',
    city_pieces: { port: 1, temple: 1, academy: 1, fortress: 0 },
    port_position: 'E4',
  };
  me.cities = [city];
  for (const water of ['C4', 'C5', 'D3', 'D5', 'E4', 'E5']) {
    city.port_position = water;
    me.units = [{ id: 0, unit_type: 'Ship', position: water }];
    await page.evaluate(({ game, view }) => window.showHarbor(game, view), {
      game,
      view: JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0)),
    });
    await page.waitForTimeout(200);
    const proof = await page.evaluate(() => window.modelProof());
    assert.ok(
      proof.every((p) => p.matchesNormalOwner && p.differsFromCity),
      JSON.stringify(proof),
    );
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
  assert.equal(await page.evaluate(() => window.shipCount()), 4);
  for (const kind of ['academy', 'market', 'obelisk', 'observatory', 'fortress', 'temple']) {
    city.city_pieces = { port: 1, [kind]: 1 };
    await page.evaluate(({ game, view }) => window.showHarbor(game, view), {
      game,
      view: JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0)),
    });
    const proof = await page.evaluate((kind) => window.modelProof([kind, 'port']), kind);
    assert.ok(
      proof.every((p) => p.matchesNormalOwner),
      JSON.stringify(proof),
    );
  }
  assert.deepEqual(errors, []);
  await page.close();
  console.log(
    'Owner-model equality (Port/Temple/Academy): passed. Foreign Port: all six shores, city picking, sea-route picking and four-ship fleet passed.',
  );
} finally {
  await browser.close();
  await server.close();
}
