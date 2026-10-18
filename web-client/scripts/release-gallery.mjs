// Exercise the actual built bundle on disposable tutorial positions; never live games.
import { chromium } from 'playwright';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const engine = createRequire(import.meta.url)('../.engine/server.js');
const out = path.join(root, 'release/screenshots');
await mkdir(out, { recursive: true });
const browser = await chromium.launch({headless:true,args:['--use-gl=angle','--use-angle=swiftshader']});
const errors = [], checks = [];
try {
  for (const dark of [false,true]) {
    const page = await browser.newPage({viewport:{width:1440,height:960},deviceScaleFactor:1});
    page.on('pageerror', e => errors.push(e.message));
    await page.route('**/*', async route => {
      const url = new URL(route.request().url());
      if(url.hostname !== 'clash.test') return route.abort();
      if(url.pathname === '/') return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>'});
      const file = path.join(root,'dist',url.pathname);
      try { return route.fulfill({body:await readFile(file),contentType:file.endsWith('.wasm')?'application/wasm':file.endsWith('.json')?'application/json':file.endsWith('.woff2')?'font/woff2':'text/javascript'}); }
      catch { return route.abort(); }
    });
    await page.goto('http://clash.test/');
    const state = await readFile(path.join(root,'../server/tests/test_games/status_phase/end_game.json'),'utf8');
    await page.evaluate(({state,dark})=>{
      window.host=clash3d.launch('#app');
      host.emit('player',{index:0});host.emit('theme',{dark});
      host.emit('preferences',{sound:false,locale:'en',mapView:'3d'});
      host.emit('state',state);
    },{state:engine.stripSecret(state,0),dark});
    await page.locator('.player-card').first().waitFor();
    await page.waitForTimeout(1000);
    const suffix=dark?'dark':'light';
    await page.screenshot({path:path.join(out,`board-${suffix}.png`)});
    await page.getByRole('button',{name:'Research tree',exact:true}).click();
    await page.waitForTimeout(500);
    await page.screenshot({path:path.join(out,`research-${suffix}.png`)});
    await page.keyboard.press('Escape');
    checks.push({dark,diagnostics:await page.evaluate(()=>clash3d.diagnostics())});
    await page.getByRole('button',{name:'Strategy map',exact:true}).click();
    await page.waitForTimeout(500);
    await page.screenshot({path:path.join(out,`strategy-${suffix}.png`)});
    await page.setViewportSize({width:390,height:844});
    await page.waitForTimeout(500);
    await page.screenshot({path:path.join(out,`mobile-${suffix}.png`)});
    checks.push({dark,mobile:await page.evaluate(()=>({width:innerWidth,scrollWidth:document.documentElement.scrollWidth}))});
    await page.close();
  }
  await writeFile(path.join(out,'checks.json'),JSON.stringify({errors,checks},null,2));
  if(errors.length)throw new Error(errors.join('\n'));
} finally {await browser.close();}
