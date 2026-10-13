// Mobile toolbar spacing and accessible Redo without an extra action column.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const npcs = JSON.parse(await engine.init(2, [], {}, 'map-targets', {})).players.slice(2);
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
const frame = (page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
async function open(width, height, state, strategy = false) {
  const page = await browser.newPage({ viewport: { width, height } }),
    errors = [],
    sent = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.exposeFunction('captureMove', (move) => sent.push(JSON.parse(move)));
  await page.route('**/*', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/')
      return route.fulfill({
        contentType: 'text/html',
        body: '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>',
      });
    if (!['/viewer.js', '/server_bg.wasm'].includes(path)) return route.abort();
    return route.fulfill({
      contentType: path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
      body: await readFile(new URL('../dist' + path, import.meta.url)),
    });
  });
  await page.goto('http://clash.test/');
  await page.evaluate(
    ({ state, strategy }) => {
      window.host = clash3d.launch('#app');
      host.on('move', (m) => window.captureMove(m));
      host.emit('player', { index: 0 });
      host.emit('preferences', { sound: false, mapView: strategy ? 'strategy' : '3d' });
      host.emit('state', state);
    },
    { state: engine.stripSecret(JSON.stringify(state), 0), strategy },
  );
  return { page, errors, sent };
}
try {
 for(const width of [320,360,393,430]) {
  const base=await fixture('advances/collect_free_economy');
  const collected=engine.tryMove(JSON.stringify(base),JSON.stringify({Playing:{Collect:{city_position:'C2',collections:[{position:'B1',pile:{ore:1},times:1}],action_type:'Collect'}}}),0);
  const g=JSON.parse(engine.tryMove(collected,JSON.stringify('Undo'),0));
  const {page,errors,sent}=await open(width,700,g);
  await page.evaluate(()=>document.querySelector('.game-shell').classList.add('dark'));
  await page.locator('.player-card').first().waitFor();
  for(const strategy of [false,true]) {
   if(strategy)await page.getByRole('button',{name:'Strategy map',exact:true}).click();
   const boxes=await page.evaluate(()=>Object.fromEntries(['.map-controls','.table-tools','.player-list','.board-toolbar'].map(s=>{const r=document.querySelector(s).getBoundingClientRect();return [s,{x:r.x,y:r.y,right:r.right,bottom:r.bottom,height:r.height}]})));
   assert(boxes['.map-controls'].height<=50,JSON.stringify(boxes));
   assert(boxes['.map-controls'].x>=0 && boxes['.map-controls'].right<=width);
   assert(boxes['.table-tools'].bottom<=boxes['.map-controls'].y-4,JSON.stringify(boxes));
   assert(boxes['.player-list'].right+9<=boxes['.board-toolbar'].x,JSON.stringify(boxes));
   await page.screenshot({path:`/tmp/mobile-toolbar-${width}-${strategy}.png`});
  }
  const undo=page.getByRole('button',{name:'Undo last action',exact:true});
  await undo.focus(); await page.keyboard.press('ArrowDown');
  await page.getByRole('button',{name:'Redo last undone action',exact:true}).waitFor();
  await page.keyboard.press('Escape');
  const box=await undo.boundingBox(); await page.mouse.move(box.x+box.width/2,box.y+box.height/2);
  await page.mouse.down(); await new Promise(r=>setTimeout(r,800)); await page.mouse.up();
  assert.equal(sent.length,0,'Long press must not also Undo');

  await page.getByRole('button',{name:'Redo last undone action',exact:true}).click();
  assert.deepEqual(sent,['Redo']);
  assert.deepEqual(errors,[]);await page.close();console.log(`${width}: controls, activity bar, and player/action spacing passed`);
 }
} finally {await browser.close();}
