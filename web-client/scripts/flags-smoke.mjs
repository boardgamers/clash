import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';

const root = new URL('..', import.meta.url).pathname;
const engine = createRequire(import.meta.url)('../.engine/server.js');
const entry = `
import {World} from '${root}/src/board.ts';
import {Controller} from '${root}/src/controller.ts';
import {CivilizationFlags} from '${root}/src/civilization-flags.ts';
import {civilizationPieceStyles} from '${root}/src/piece-styles.ts';
import {get} from 'svelte/store';
import * as THREE from 'three';
import '${root}/src/style.css';
let world, session;
window.show=(game,view)=>{
  world?.destroy();
  session={...get(new Controller({},new URL(location.href)).session),game,view,seat:0};
  world=new World(document.querySelector('#app'),()=>{});
  world.update(session);
};
window.change=patch=>{session={...session,...patch};world.update(session)};
window.rotate=()=>{
  const offset=world.camera.position.clone().sub(world.controls.target);
  offset.applyAxisAngle(new THREE.Vector3(0,1,0),Math.PI);
  world.camera.position.copy(world.controls.target).add(offset);
  world.controls.update();world.invalidate();
};
window.zoom=()=>world.zoom(.68);
window.flags=()=>{
 world.scene.updateMatrixWorld(true); world.camera.updateMatrixWorld();
 return world.pieces.filter(p=>p.userData.kind==='city').map(city=>{
  const flag=city.children.find(c=>Array.isArray(c.material));
  const texture=flag.material[4].map;
  const box=new THREE.Box3().setFromObject(flag);
  const corners=[];for(const x of [box.min.x,box.max.x])for(const y of [box.min.y,box.max.y])for(const z of [box.min.z,box.max.z]){
   const p=new THREE.Vector3(x,y,z).project(world.camera);
   corners.push({x:(p.x+1)*innerWidth/2,y:(1-p.y)*innerHeight/2});
  }
  return {version:texture.version,texture:texture.uuid,back:flag.material[5].map.uuid,
   left:Math.min(...corners.map(p=>p.x)),right:Math.max(...corners.map(p=>p.x)),
   top:Math.min(...corners.map(p=>p.y)),bottom:Math.max(...corners.map(p=>p.y))};
 });
};
window.gallery=async()=>{
 const flags=new CivilizationFlags(()=>{});
 const textures=Object.keys(civilizationPieceStyles).concat('Barbarians','Pirates').map(civ=>({civ,texture:flags.texture(civ,civ==='Barbarians'?'#bfa986':'#5086af')}));
 await new Promise(r=>setTimeout(r,500));
 const report=textures.map(({civ,texture})=>({civ,loaded:texture.version>1}));
 const gallery=document.createElement('div');gallery.id='gallery';
 gallery.style.cssText='position:fixed;inset:0;z-index:100;background:#dae2d9;display:grid;grid-template-columns:repeat(6,1fr);gap:12px;padding:16px;color:#243a32';
 textures.forEach(({civ,texture})=>{const cell=document.createElement('div');cell.textContent=civ;const img=new Image();img.src=texture.image.toDataURL();img.style.cssText='display:block;width:144px;height:96px;margin-top:8px';cell.append(img);gallery.append(cell)});
 document.body.append(gallery); flags.dispose();return report;
};
window.ready=true;`;
const server = await createServer({
  root,
  configFile: false,
  publicDir: false,
  plugins: [
    svelte(),
    {
      name: 'isolated-flag-fixture',
      resolveId: (id) => (id === '/flags-entry.js' ? '\0flags-entry' : null),
      load: (id) => (id === '\0flags-entry' ? entry : null),
      configureServer(server) {
        server.middlewares.use(async (req, res, next) => {
          if (req.url !== '/flags') return next();
          res.setHeader('Content-Type', 'text/html');
          res.end(
            await server.transformIndexHtml(
              req.url,
              '<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><style>body{margin:0}#app{height:100vh;width:100vw}</style><div class="play-layout"><div id="app"></div></div><script type="module" src="/flags-entry.js"></script>',
            ),
          );
        });
      },
    },
  ],
  server: { host: '127.0.0.1', port: 0 },
});
await server.listen();
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  let state = await engine.init(2, [], { civilization: 'ChooseCivilization' }, 'flag-fixture', {});
  for (const civ of ['China', 'Maya'])
    state = engine.tryMove(state, JSON.stringify({ ChooseCivilization: civ }), engine.currentPlayer(state));
  const game = JSON.parse(state);
  game.map.tiles = [];
  for (const col of ['B', 'C', 'D', 'E', 'F'])
    for (let row = 2; row <= 6; row++) game.map.tiles.push([col + row, 'Fertile']);
  for (const p of game.players) {
    p.cities = [];
    p.units = [];
  }
  const barbarian = game.players.find((p) => p.civilization === 'Barbarians');
  for (const [player, position, mood_state] of [
    [game.players[0], 'C3', 'Happy'],
    [game.players[1], 'E3', 'Neutral'],
    [barbarian, 'D5', 'Angry'],
  ])
    player.cities = [{ position, mood_state, city_pieces: {} }];
  const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  for (const width of [1000, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 800 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(server.resolvedUrls.local[0] + 'flags');
    await page.waitForFunction(() => window.ready);
    await page.evaluate(
      ({ game, view }) => {
        window.show(game, view);
        window.change({ playerColors: ['#356083', '#e1c876'] });
      },
      { game, view },
    );
    await page.waitForFunction(() => window.flags().every((f) => f.version > 1));
    for (const side of ['front', 'back']) {
      await page.evaluate(() => window.zoom());
      await page.waitForTimeout(200);
      await page.screenshot({ path: '/tmp/clash-flags-' + width + '-' + side + '.png' });
      const flags = await page.evaluate(() => window.flags());
      assert.ok(
        flags.every((f) => f.texture === f.back),
        'Emblem appears on both faces',
      );
      const labels = await page.locator('.city-map-label').evaluateAll((nodes) =>
        nodes.map((n) => {
          const r = n.getBoundingClientRect();
          return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
        }),
      );
      flags.forEach((f, i) =>
        assert.ok(
          labels[i].right <= f.left || labels[i].bottom <= f.top || labels[i].top >= f.bottom,
          'Mood clears the larger flag',
        ),
      );
      await page.evaluate(() => window.rotate());
    }
    const before = await page.evaluate(() => window.flags().map((f) => f.texture));
    await page.evaluate(() => {
      window.change({ strategyMap: true, topDown: true });
      window.change({ strategyMap: false, topDown: false });
    });
    assert.deepEqual(
      await page.evaluate(() => window.flags().map((f) => f.texture)),
      before,
      'Reuse loaded textures when returning from Strategy',
    );
    await page.evaluate(() => window.change({ colorBlind: true, playerSymbols: ['star', 'hexagon'] }));
    await page.waitForFunction(() => window.flags().every((f) => f.version > 1));
    await page.waitForTimeout(200);
    await page.screenshot({ path: '/tmp/clash-flags-' + width + '-accessible.png' });
    if (width === 1000) {
      const report = await page.evaluate(() => window.gallery());
      assert.ok(
        report.every((r) => r.loaded),
        JSON.stringify(report),
      );
      await page.screenshot({ path: '/tmp/clash-flags-emblems.png' });
    }
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log(
    'All civilization emblems decode; front/back flags, custom colors, mood clearance, mobile and Strategy rebuilds verified.',
  );
} finally {
  await browser.close();
  await server.close();
}
