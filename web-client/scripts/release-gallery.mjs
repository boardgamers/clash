// Read-only public completed game; offline viewer. Never play or edit a live game.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const engine = createRequire(import.meta.url)('../.engine/server.js');
const out = path.join(root, 'release/screenshots');
const gameId = 'Open-clash-a52a21ee';
// Anonymous GET only. Do not save the response: completed games can include hands.
const response = await fetch(`https://boardgamers.space/api/gameplay/${gameId}`, { method: 'GET' });
assert(response.ok, `Public game GET failed (${response.status})`);
const source = await response.json();
assert.equal(source.status, 'ended');
const game = JSON.parse(engine.stripSecret(source.data, undefined));
delete source.data;
const boardHash = createHash('sha256').update(JSON.stringify({map:game.map,players:game.players.map(({cities,units})=>({cities,units}))})).digest('hex');
// Names and free-text history are unnecessary for a gallery. Keep all actual
// map tiles, cities, buildings, units, public advances, scores and outcome.
for (const p of game.players) if (p.name) p.name = p.civilization;
game.messages = []; game.log = []; game.board_history = { id: 'release-gallery', frames: [] };
delete game.custom_ui; delete game.test_dice;
assert.equal(boardHash, '411f62a3c258bc1c73e7a5ebebefcc567c1d66e29b602f8675c73202de5b3e05', 'Source board changed; review provenance before recapturing');
const state = JSON.stringify(game); // In memory only; never persist production state.
JSON.parse(engine.webView(state, undefined)); // Fail before capture on incompatible state.
await mkdir(out, { recursive: true });
const browser = await chromium.launch({headless:true,args:['--use-gl=angle','--use-angle=swiftshader']});
const errors = [], checks = [];
try {
  for (const dark of [false,true]) {
    const page = await browser.newPage({viewport:{width:1440,height:960},deviceScaleFactor:1});
    page.on('pageerror', e => errors.push(e.message));
    await page.route('**/*', async route => {
      const url = new URL(route.request().url());
      if(url.hostname !== 'clash.test' || route.request().method() !== 'GET') return route.abort();
      if(url.pathname === '/') return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>'});
      const file = path.join(root,'dist',url.pathname);
      try { return route.fulfill({body:await readFile(file),contentType:file.endsWith('.wasm')?'application/wasm':file.endsWith('.json')?'application/json':file.endsWith('.woff2')?'font/woff2':'text/javascript'}); }
      catch { return route.abort(); }
    });
    await page.goto('http://clash.test/');
    await page.evaluate(({state,dark})=>{
      window.host=clash3d.launch('#app'); window.sent=[];
      host.on('move', move => sent.push(move));
      let preferences={sound:false,locale:'en',mapView:'3d',replayAutoplay:false};
      host.on('update:preference', ({name,value})=>{preferences={...preferences,[name]:value};host.emit('preferences',preferences)});
      host.emit('player',{});host.emit('theme',{dark});
      host.emit('preferences',preferences);host.emit('state',state);
    },{state,dark});
    await page.locator('.player-card').first().waitFor();
    await page.getByText('Loading game…',{exact:true}).waitFor({state:'hidden'});
    await page.waitForTimeout(1500);
    await page.keyboard.press('Escape');
    const suffix=dark?'dark':'light';
    await page.screenshot({path:path.join(out,`board-${suffix}.png`)});
    // The research browser requires a selected civilization. This changes only
    // the local viewing seat; the payload remains spectator-stripped (no hand).
    await page.evaluate(()=>host.emit('player',{index:0}));
    await page.getByRole('button',{name:'Research tree',exact:true}).click();
    await page.locator('.research-tree').waitFor();
    await page.waitForTimeout(500);
    await page.screenshot({path:path.join(out,`research-${suffix}.png`)});
    await page.keyboard.press('Escape');
    await page.evaluate(()=>host.emit('player',{}));
    checks.push({dark,diagnostics:await page.evaluate(()=>clash3d.diagnostics())});
    await page.getByRole('button',{name:'Strategy map',exact:true}).click();
    await page.waitForTimeout(700);
    await page.screenshot({path:path.join(out,`strategy-${suffix}.png`)});
    await page.setViewportSize({width:390,height:844});
    await page.waitForTimeout(700);
    // Pan with the real strategy-map controls to clear the stacked player cards.
    await page.mouse.move(350,350); await page.mouse.down({button:'right'});
    await page.mouse.move(350,395,{steps:12}); await page.mouse.up({button:'right'});
    await page.mouse.move(389,110);
    await page.waitForTimeout(700);
    await page.screenshot({path:path.join(out,`mobile-${suffix}.png`)});
    const mobile=await page.evaluate(()=>({width:innerWidth,scrollWidth:document.documentElement.scrollWidth,moves:sent.length}));
    assert.equal(mobile.moves,0); assert.equal(mobile.width,mobile.scrollWidth);
    checks.push({dark,mobile});
    await page.close();
  }
  await writeFile(path.join(out,'checks.json'),JSON.stringify({source:`https://boardgamers.space/game/${gameId}`,status:source.status,capturedAt:new Date().toISOString(),boardHash,viewerVersion:JSON.parse(await readFile(path.join(root,'package.json'),'utf8')).version,errors,checks},null,2)+'\n');
  if(errors.length)throw new Error(errors.join('\n'));
} finally {await browser.close();}
