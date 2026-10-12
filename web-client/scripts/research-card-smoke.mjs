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
  const state=await fixture('action_cards/synergies');
  state.players[0].action_cards=[33];
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
  await page.getByRole('button',{name:'Research tree',exact:true}).click();
  const hint=page.getByRole('button',{name:'Use Tech Synergy · 2 advances · normal resource costs'}).first();
  await hint.waitFor();
  await hint.scrollIntoViewIfNeeded();
  await page.screenshot({path:`/tmp/clash-research-cards-${width}.png`});
  await page.getByRole('button',{name:'Details of Tech Synergy',exact:true}).first().click();
  await page.getByRole('dialog',{name:'Tech Synergy rules',exact:true}).waitFor();
  await page.getByRole('button',{name:'Close card rules',exact:true}).click();
  await hint.click();
  await page.waitForFunction(() => sent.length === 1);
  const rome = await fixture('civilizations/rome/aqueduct_free_action');
  rome.actions_left = 0;
  await page.reload();
  await page.evaluate(state => {
    window.host=clash3d.launch('#app'); window.sent=[];
    host.on('move',a=>sent.push(a)); host.emit('player',{index:0});
    host.emit('theme',{dark:true}); host.emit('preferences',{sound:false}); host.emit('state',state);
  },engine.stripSecret(JSON.stringify(rome),0));
  await page.getByRole('button',{name:'Research tree',exact:true}).click();
  await page.locator('#research-Sanitation .research-pick').click();
  const free = page.getByRole('button',{name:/Research Sanitation · free action/});
  assert.equal(await free.isEnabled(),true);
  await free.click();
  await page.waitForFunction(() => sent.length === 1);
  assert.deepEqual(errors,[]);
  await page.close();
 }
 console.log('Research card desktop/mobile smoke passed');
} finally {await browser.close();}
