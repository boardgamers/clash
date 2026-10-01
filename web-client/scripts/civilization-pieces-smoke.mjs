import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { build } from 'esbuild';
import { chromium } from 'playwright';
const root = new URL('..', import.meta.url).pathname;
const bundled = await build({
  stdin: {
    resolveDir: root,
    loader: 'ts',
    contents: `
import * as THREE from 'three';
import {PieceModels} from './src/piece-models';
import {civilizationPieceStyles} from './src/piece-styles';
import {unitPortrait} from './src/unit-portrait';
const renderer = new THREE.WebGLRenderer({alpha:true,antialias:true}); renderer.setSize(190,164,false);renderer.setClearColor(0x000000,0);
renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.05;
const civs=Object.keys(civilizationPieceStyles), kinds=['settlement','temple','Leader','Infantry','Settler','Cavalry','Elephant','Ship'];
window.report=[];
window.draw=(start=0, buildings=false)=>{
 document.querySelector('main')?.remove();const main=document.createElement('main');document.body.append(main);
 const types=buildings?['settlement','academy','market','temple','fortress','observatory','obelisk','port']:kinds;
 const cells=['Civilization',...types];for(const name of cells){const label=document.createElement('header');label.textContent=name;main.append(label);}
 for(const civilization of civs.slice(start,start+5)){
  const label=document.createElement('h2');label.textContent=civilization;main.append(label);
  for(const kind of types){
   const materials=[],geometries=[];
   const models=new PieceModels(color=>{const m=new THREE.MeshStandardMaterial({color,roughness:1});materials.push(m);return m},(geometry,material)=>{geometries.push(geometry);return new THREE.Mesh(geometry,material)});
   const model=kind==='settlement'?models.settlement('#5086af',civilization):kind[0]===kind[0].toLowerCase()?models.building(kind,'#5086af',civilization):models.unit(kind==='Leader'?{Leader:'Leader'}:kind,'#5086af',false,civilization);
   const bounds=new THREE.Box3().setFromObject(model), center=bounds.getCenter(new THREE.Vector3()), radius=bounds.getBoundingSphere(new THREE.Sphere()).radius;
   const size=Math.max(.55,radius*1.12), camera=new THREE.OrthographicCamera(-size*190/164,size*190/164,size,-size,.1,20);camera.position.copy(center).add(new THREE.Vector3(2.6,1.9,3.4));camera.lookAt(center);
   const scene=new THREE.Scene();scene.add(model,new THREE.HemisphereLight('#fff4d8','#386775',2.2));const sun=new THREE.DirectionalLight('#ffe0a7',3.2);sun.position.set(-9,18,8);scene.add(sun);renderer.render(scene,camera);
   const image=document.createElement('img');image.src=renderer.domElement.toDataURL();image.alt=civilization+' '+kind;main.append(image);
   window.report.push({civilization,kind,meshes:geometries.length,height:bounds.max.y-bounds.min.y,width:bounds.max.x-bounds.min.x,minY:bounds.min.y,finite:Number.isFinite(radius),colors:[...new Set(materials.map(m=>'#'+m.color.getHexString()))]});
   geometries.forEach(g=>g.dispose());materials.forEach(m=>m.dispose());
  }
 }
};
window.checkPortraits=()=>{
 const greek=unitPortrait('Infantry','#5086af',false,'Greece'), roman=unitPortrait('Infantry','#5086af',false,'Rome');
 return {different:greek!==roman,cached:greek===unitPortrait('Infantry','#5086af',false,'Greece'),color:greek!==unitPortrait('Infantry','#b96c4c',false,'Greece'),pirate:unitPortrait('Ship','#5086af',true,'Greece')!==unitPortrait('Ship','#5086af',false,'Greece')};
};
window.draw();window.ready=true;
`,
  },
  bundle: true,
  write: false,
  format: 'iife',
  logLevel: 'silent',
});
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
  const page = await browser.newPage({ viewport: { width: 1660, height: 960 } }),
    errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.route('**/*', (route) => {
    if (new URL(route.request().url()).pathname === '/gallery.js')
      return route.fulfill({ contentType: 'text/javascript', body: bundled.outputFiles[0].text });
    return route.fulfill({
      contentType: 'text/html',
      body: '<!doctype html><meta charset="utf-8"><style>body{margin:0;padding:16px;background:#d7dfd6;color:#253c35;font:14px system-ui} main{display:grid;grid-template-columns:100px repeat(8,190px);align-items:center;gap:10px 0} header{padding:8px;text-align:center;font-weight:600;text-transform:capitalize} h2{font-size:16px;overflow-wrap:anywhere} img{width:190px;height:164px;background:#ffffff36;border-radius:8px} </style><body><script src="/gallery.js"></script>',
    });
  });
  await page.goto('http://clash-gallery.test/');
  await page.waitForFunction(() => window.ready);
  for (const buildings of [false, true])
    for (const start of [0, 5, 10]) {
      await page.evaluate(({ start, buildings }) => window.draw(start, buildings), { start, buildings });
      await page.screenshot({
        path: `/tmp/clash-civilization-${buildings ? 'buildings' : 'pieces'}-${start}.png`,
      });
    }
  const report = await page.evaluate(() => window.report);
  assert.equal(new Set(report.map((r) => r.civilization)).size, 15);
  for (const piece of report) {
    assert.ok(
      piece.finite && piece.height > 0.2 && piece.height < 1.6 && piece.width < 1.4 && piece.minY > -0.001,
      JSON.stringify(piece),
    );
    assert.ok(piece.meshes < 85, JSON.stringify(piece));
    assert.ok(
      piece.colors.includes('#5086af'),
      `Ownership color missing on ${piece.civilization} ${piece.kind}`,
    );
  }
  assert.deepEqual(await page.evaluate(() => window.checkPortraits()), {
    different: true,
    cached: true,
    color: true,
    pirate: true,
  });
  assert.deepEqual(errors, []);
  await writeFile('/tmp/clash-civilization-pieces-report.json', JSON.stringify(report, null, 2));
  console.log(
    'All 15 civilizations: architecture and unit renders, geometry bounds, ownership colors, mesh budgets, portrait cache variants.',
  );
} finally {
  await browser.close();
}
