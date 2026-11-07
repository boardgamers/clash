import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
const engine = createRequire(import.meta.url)('../.engine/server.js');
const server = createServer(async (req, res) => {
  if (req.url === '/') return res.end('<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>');
  if (!/^\/(viewer.js|server_bg.wasm|de.json)$/.test(req.url)) return res.writeHead(404).end();
  res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : req.url.endsWith('.json') ? 'application/json' : 'text/javascript');
  res.end(await readFile(new URL('../dist' + req.url, import.meta.url)));
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({headless:true, args:['--use-gl=angle', '--use-angle=swiftshader']});
try {
  for (const locale of ['en','de']) for (const width of [1400,390,320]) for (const id of [31,32]) {
    const game = JSON.parse(await engine.init(2, [], {}, `goals-${id}`, {}));
    const seat = engine.currentPlayer(JSON.stringify(game));
    game.players[seat].action_cards = [id];
    game.players[seat].resources.culture_tokens = 5;
    game.players[seat].objective_cards.push(game.objective_cards_left.pop());
    const move = (state, action) => engine.tryMove(state, typeof action === 'string' ? action : JSON.stringify(action), seat);
    const initial = move(move(JSON.stringify(game), {Playing:{ActionCard:id}}), {Response:{Payment:[{culture_tokens:1}]}});
    let state = initial, sent = [];
    const page = await browser.newPage({viewport:{width,height:width===1400?900:844}}), errors=[];
    page.on('pageerror', error => errors.push(error.message));
    await page.exposeFunction('applyMove', async action => {
      sent.push(typeof action === 'string' ? JSON.parse(action) : action);
      state=move(state, action);
      await page.evaluate(s => host.emit('state',s), engine.stripSecret(state,seat));
    });
    await page.goto('http://127.0.0.1:'+server.address().port);
    await page.evaluate(({state,seat,locale}) => {
      window.host=clash3d.launch('#app');host.on('move', a=>window.applyMove(a));
      host.emit('player',{index:seat});host.emit('preferences',{sound:false,locale});host.emit('state',state);
    },{state:engine.stripSecret(state,seat),seat,locale});
    await page.waitForSelector(`[data-clash-locale="${locale}"] .objective-exchange-footer`);
    const panel=page.locator('.card-decision'), options=panel.locator('.decision-option');
    const exchange=panel.locator('.objective-exchange-footer .primary'), keep=panel.locator('.objective-exchange-footer .secondary');
    assert.equal(await panel.locator('.decision-card-group').count(),2);
    assert.equal(await panel.locator('.decision-card-face').count(),8);
    assert.equal(await panel.locator('.decision-count').count(),0);
    assert.equal(await exchange.isEnabled(),false);
    assert.equal(await keep.isEnabled(),true);
    await keep.click();
    await page.waitForFunction(()=>!document.querySelector('.card-decision'));
    assert.deepEqual(sent.at(-1), {Response:{SelectHandCards:[]}});
    state=initial;
    await page.evaluate(s=>host.emit('state',s),engine.stripSecret(state,seat));
    await panel.waitFor();
    await options.nth(0).click(); assert.equal(await exchange.isEnabled(),false);
    await options.nth(1).click(); assert.equal(await options.nth(0).getAttribute('aria-pressed'),'false');
    await options.nth(2).click(); assert.equal(await exchange.isEnabled(),true);
    await options.nth(3).click(); assert.equal(await options.nth(2).getAttribute('aria-pressed'),'false');
    assert.equal(await panel.locator('.exchange-selection').count(),2);
    assert.equal(await panel.locator('.inline-error').count(),0);
    assert.ok(await panel.evaluate(el=>el.scrollWidth<=el.clientWidth+1));
    assert.ok(await exchange.evaluate(el=>{const r=el.getBoundingClientRect();return r.left>=0 && r.right<=innerWidth+1 && el.scrollWidth<=el.clientWidth+1}));
    await panel.evaluate(el=>el.scrollTop=0);
    await page.screenshot({path:`/tmp/clash-new-goals-${locale}-${width}-${id}.png`});
    // Keeping is independent of an otherwise valid highlighted exchange.
    await keep.click();
    await page.waitForFunction(()=>!document.querySelector('.card-decision'));
    assert.deepEqual(sent.at(-1), {Response:{SelectHandCards:[]}});
    state=initial;
    await page.evaluate(s=>host.emit('state',s),engine.stripSecret(state,seat));
    await panel.waitFor();
    await options.nth(1).click();
    await options.nth(3).click();
    await exchange.click();
    await page.waitForFunction(()=>!document.querySelector('.card-decision'));
    assert.equal(sent.at(-1).Response.SelectHandCards.length,2);
    assert.deepEqual(errors,[]);
    console.log(`${locale} ${width}px card ${id}: groups, faces, switching, keep, exchange, fit passed`);
    await page.close();
  }
} finally { await browser.close();server.close(); }
