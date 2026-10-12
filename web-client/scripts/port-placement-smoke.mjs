import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'abilities-ui', {})).players.slice(2);
async function fixture(name) {
  const game = JSON.parse(
    await readFile(new URL(`../../server/tests/test_games/${name}.json`, import.meta.url), 'utf8'),
  );
  for (const npc of npcs)
    if (!game.players.some((p) => p.civilization === npc.civilization))
      game.players.push({ ...npc, id: game.players.length });
  return game;
}
const browser = await chromium.launch({
  headless: true,
  args: ['--use-gl=angle', '--use-angle=swiftshader'],
});
try {
 for (const [width,height] of [[1400,900],[390,740]]) {
  const state=await fixture('base/construct_port');
  const page=await browser.newPage({viewport:{width,height}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/*',async route=>{
   const path=new URL(route.request().url()).pathname;
   if(path==='/') return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>'});
   if(!['/viewer.js','/server_bg.wasm'].includes(path)) return route.abort();
   return route.fulfill({contentType:path.endsWith('.wasm')?'application/wasm':'text/javascript',body:await readFile(new URL('../dist'+path,import.meta.url))});
  });
  await page.goto('http://clash.test');
  await page.evaluate(state=>{window.host=clash3d.launch('#app');window.sent=[];host.on('move',a=>sent.push(a));host.emit('player',{index:0});host.emit('theme',{dark:true});host.emit('preferences',{sound:false});host.emit('state',state)},engine.stripSecret(JSON.stringify(state),0));
  await page.getByRole('button',{name:'Manage cities',exact:true}).click();
  await page.getByRole('navigation',{name:'Choose city'}).getByRole('button',{name:/C2/}).click();
  await page.locator('.building-grid button').filter({hasText:'Port'}).click();
  await page.getByRole('button',{name:'Choose port location on map',exact:true}).click();
  await page.getByRole('region',{name:'Choose port location'}).waitFor();
  assert.equal(await page.locator('.city-dialog').count(),0);
  await page.screenshot({path:`/tmp/clash-port-map-${width}.png`});
  const target=page.locator('.map-hit-target[data-position="D2"]');
  await target.waitFor({state:'attached'});
  await page.waitForTimeout(400);
  const box=await target.boundingBox();assert(box);
  await page.mouse.click(box.x+box.width/2,box.y+box.height/2);
  await page.waitForFunction(()=>sent.length===1);
  const action=await page.evaluate(()=>sent[0]);
  const after=JSON.parse(engine.tryMove(JSON.stringify(state),typeof action==='string'?action:JSON.stringify(action),0));
  assert.equal(after.players[0].cities.find(c=>c.position==='C2').port_position,'D2');
  assert.deepEqual(errors,[]);
  await page.close();
 }
 console.log('Port placement desktop/mobile smoke passed');
} finally {await browser.close();}
