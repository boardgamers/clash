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
  const state=await fixture('advances/roads_unit_test');
  state.current_player_index=1;
  state.players[1].units=state.players[1].units.filter(u=>u.id!==2);
  state.players[1].resources={food:1,ore:1,gold:2};
  const target=state.players[1].cities.find(c=>c.position==='F7');
  state.players[1].cities=state.players[1].cities.filter(c=>c.position!=='F7');
  state.players[0].cities.push(target);
  const page=await browser.newPage({viewport:{width,height}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/*',async route=>{
   const path=new URL(route.request().url()).pathname;
   if(path==='/') return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>'});
   if(!['/viewer.js','/server_bg.wasm'].includes(path)) return route.abort();
   return route.fulfill({contentType:path.endsWith('.wasm')?'application/wasm':'text/javascript',body:await readFile(new URL('../dist'+path,import.meta.url))});
  });
  await page.goto('http://clash.test');
  await page.evaluate(state=>{window.host=clash3d.launch('#app');window.sent=[];host.on('move',a=>sent.push(a));host.emit('player',{index:1});host.emit('theme',{dark:true});host.emit('preferences',{sound:false});host.emit('state',state)},engine.stripSecret(JSON.stringify(state),1));
  await page.waitForTimeout(500);
  await page.waitForTimeout(400);
  async function tile(pos) {
    const box=await page.locator(`.map-hit-target[data-position="${pos}"]`).boundingBox();
    assert(box); await page.mouse.click(box.x+box.width/2,box.y+box.height/2); await page.waitForTimeout(250);
  }
  await tile('D8');
  await page.screenshot({path:`/tmp/move-before-target-${width}.png`});
  await tile('F7');
  const choices=page.getByRole('group',{name:'Choose how to move'});
  await choices.waitFor();
  assert.equal(await choices.getByRole('button').count(),4);
  assert.deepEqual(await page.evaluate(()=>sent),[]);
  await choices.getByRole('button').first().click();
  await page.getByLabel('Attack preview',{exact:true}).waitFor();
  assert.deepEqual(await page.evaluate(()=>sent),[]);
  const bounds=await page.locator('[aria-label="Unit movement"]').boundingBox();
  const attack=await page.locator('.settler-confirm button').boundingBox();
  assert.ok(bounds && attack && attack.y >= bounds.y && attack.y + attack.height <= bounds.y + bounds.height, 'attack stays visible');
  assert.equal(await page.getByLabel('Attack preview',{exact:true}).locator('h3').innerText(),'Defenders');
  await page.screenshot({path:`/tmp/clash-roads-payment-${width}.png`});
  assert.deepEqual(errors,[]);
  await page.close();
 }
 console.log('Roads payment selection before attack desktop/mobile smoke passed');
} finally {await browser.close();}
