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
  const state=await fixture('advances/trade_routes');
  state.current_player_index=1;
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
  await page.getByRole('button',{name:'Show trade routes',exact:true}).click();
  const guide=page.locator('.pirate-spawn-key').filter({has:page.getByRole('combobox',{name:'Trade route player'})});
  await guide.getByText('Current eligible pairings · 2',{exact:true}).waitFor();
  const box=await guide.boundingBox();assert(box.x>=0 && box.x+box.width<=width+1);
  await page.screenshot({path:`/tmp/clash-trade-guide-${width}.png`});
  await guide.getByRole('combobox').selectOption('0');
  await guide.getByText('Requires Trade Routes.',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Show all sea routes',exact:true}).click();
  assert.equal(await guide.count(),0);assert.deepEqual(await page.evaluate(()=>sent),[]);assert.deepEqual(errors,[]);
  await page.close();
 }
 console.log('Trade Routes desktop/mobile smoke passed');
} finally {await browser.close();}
