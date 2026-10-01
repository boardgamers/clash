import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { chromium } from 'playwright';
const root = new URL('..', import.meta.url).pathname;
const bundle = await build({
  stdin: {
    resolveDir: root,
    loader: 'ts',
    contents: `
import * as THREE from 'three';
import { PieceModels } from './src/piece-models';
const kinds=['Pyramids','GreatGardens','Colosseum','GreatLibrary','GreatLighthouse','GreatMausoleum','GreatStatue','GreatWall'];
const renderer=new THREE.WebGLRenderer({antialias:true,alpha:true});renderer.setSize(290,250);renderer.setClearColor(0,0);
renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.05;
window.report=[];
for(const kind of kinds){
 const materials=[],geometries=[];
 const models=new PieceModels(c=>{const m=new THREE.MeshStandardMaterial({color:c,flatShading:true});materials.push(m);return m},(g,m)=>{geometries.push(g);return new THREE.Mesh(g,m)});
 const model=models.wonder(kind,'#4f83ad');
 const bounds=new THREE.Box3().setFromObject(model, true),center=bounds.getCenter(new THREE.Vector3());
 const scene=new THREE.Scene();scene.add(model,new THREE.HemisphereLight('#fff4d8','#386775',2.2));
 const sun=new THREE.DirectionalLight('#ffe0a7',3.2);sun.position.set(-9,18,8);scene.add(sun);
 const camera=new THREE.OrthographicCamera(-.94,.94,.81,-.81,.1,20);camera.position.copy(center).add(new THREE.Vector3(2.6,1.9,3.4));camera.lookAt(center);renderer.render(scene,camera);
 const cell=document.createElement('article'),image=document.createElement('img'),label=document.createElement('h2');image.src=renderer.domElement.toDataURL();image.alt=kind;label.textContent=kind.replace(/([a-z])([A-Z])/g,'$1 $2');cell.append(image,label);document.querySelector('main').append(cell);
 window.report.push({kind,meshes:geometries.length,minY:bounds.min.y,width:bounds.max.x-bounds.min.x,height:bounds.max.y-bounds.min.y,depth:bounds.max.z-bounds.min.z,image:image.src});
 geometries.forEach(g=>g.dispose());materials.forEach(m=>m.dispose());
}
window.ready=true;
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
  const page = await browser.newPage({ viewport: { width: 1240, height: 665 } }),
    errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.route('**/*', (route) =>
    route.fulfill({
      contentType: route.request().url().endsWith('.js') ? 'text/javascript' : 'text/html',
      body: route.request().url().endsWith('.js')
        ? bundle.outputFiles[0].text
        : '<!doctype html><style>body{margin:0;padding:20px;background:#d7dfd6;color:#253c35;font:14px system-ui}main{display:grid;grid-template-columns:repeat(4,1fr);gap:16px}article{text-align:center;background:#ffffff36;border-radius:8px}img{width:290px;height:250px}h2{font-size:17px;margin:4px 0 18px}</style><main></main><script src="/gallery.js"></script>',
    }),
  );
  await page.goto('http://clash-wonders.test');
  await page.waitForFunction(() => window.ready);
  await page.screenshot({ path: '/tmp/clash-wonders-gallery.png' });
  const report = await page.evaluate(() => window.report);
  assert.equal(new Set(report.map((r) => r.image)).size, 8);
  for (const p of report)
    assert(
      p.minY >= -0.001 && p.height < 1.4 && p.width < 1.15 && p.depth < 0.9 && p.meshes < 85,
      JSON.stringify({ ...p, image: undefined }),
    );
  assert.deepEqual(errors, []);
  await page.screenshot({ path: '/tmp/clash-wonders-gallery.png' });
  console.log('Eight distinct wonder renders; grounded geometry, bounded footprints, mesh budgets verified.');
} finally {
  await browser.close();
}
