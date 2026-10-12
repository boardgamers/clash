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
  const state=await fixture('movement/ship_embark');
  state.players[0].advances.push('Tactics', 'Roads');
  state.players[0].units=state.players[0].units.filter(u=>[0,3,4,8,9].includes(u.id));
  state.players[0].units.find(u=>u.id===0).position='B3';
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
  const panel=page.getByRole('region',{name:'Unit movement',exact:true});
  await panel.waitFor();
  async function tap(position) {
    const target=page.locator(`.map-hit-target[data-position="${position}"]`);
    await target.waitFor({state:'attached'});
    await page.waitForTimeout(400);
    const box=await target.boundingBox();
    assert(box);
    await page.mouse.click(box.x+box.width/2,box.y+box.height/2);
  }
  await tap('B3');
  await page.screenshot({path:`/tmp/clash-embark-before-${width}.png`});
  assert.match(await panel.innerText(), /3 selected/);
  await tap('C3');
  assert.equal(await panel.getByRole('button',{name:/Board all/}).count(),0);
  let raw=JSON.stringify(state);
  for(let i=0;i<2;i++) {
    await page.waitForFunction(i=>sent.length>i,i);
    const action=await page.evaluate(i=>sent[i],i);
    raw=engine.tryMove(raw,typeof action==='string'?action:JSON.stringify(action),0);
    await page.evaluate(state=>host.emit('state',state),engine.stripSecret(raw,0));
  }
  const after=JSON.parse(raw);
  const passengers=after.players[0].units.flatMap(u=>u.carried_units??[]);
  assert.deepEqual(passengers.map(u=>u.id).sort(),[0,3,4]);
  assert.equal(after.state.Movement.movement_actions_left,state.state.Movement.movement_actions_left-1);
  await page.screenshot({path:`/tmp/clash-embark-after-${width}.png`});
  assert.deepEqual(errors,[]);
  await page.close();
 }
 console.log('Multi-ship boarding desktop/mobile smoke passed');
} finally {await browser.close();}
