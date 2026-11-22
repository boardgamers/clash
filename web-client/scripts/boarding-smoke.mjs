import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { engine, fixture } from './boarding-fixture.mjs';
// Existing cargo must count, even when the client forges an oversized load.
{
  const state = fixture(false, 3);
  state.players[0].units.find(u => u.id === 10).carried_units = [{id:9,unit_type:'Infantry'}];
  const action = {Movement:{Move:{units:[0,1,2],destination:'F6',boarding:[
    {units:[0],destination:'F6',embark_carrier_id:10},
    {units:[1,2],destination:'F6',embark_carrier_id:11},
  ]}}};
  const after = JSON.parse(engine.tryMove(JSON.stringify(state),JSON.stringify(action),0));
  assert.deepEqual(after.players[0].units.filter(u => u.unit_type === 'Ship').map(u => u.carried_units.length),[2,2]);
  action.Movement.Move.boarding[0].units = [0,1];
  action.Movement.Move.boarding[1].units = [2];
  assert.throws(() => engine.tryMove(JSON.stringify(state),JSON.stringify(action),0));
  state.players[0].units.find(u => u.id === 10).carried_units.push({id:8,unit_type:'Infantry'});
  assert.throws(() => engine.tryMove(JSON.stringify(state),JSON.stringify(action),0));
}
const browser = await chromium.launch({headless:true,args:['--use-gl=angle','--use-angle=swiftshader']});
try {
  for (const width of [1400,390]) for (const pirates of [false,true]) for (const count of [2,4]) {
    const state = fixture(pirates,count), sent = [], errors = [];
    const page = await browser.newPage({viewport:{width,height:900}});
    page.on('pageerror', e => { errors.push(e.message); console.error(e.message); });
    await page.exposeFunction('captureMove', move => sent.push(JSON.parse(move)));
    await page.route('**/*', async route => {
      const path = new URL(route.request().url()).pathname;
      if (path === '/') return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="app"></div><script src="/viewer.js"></script>'});
      if (!['/viewer.js','/server_bg.wasm'].includes(path)) return route.abort();
      return route.fulfill({contentType:path.endsWith('.wasm') ? 'application/wasm':'text/javascript',body:await readFile(new URL('../dist'+path,import.meta.url))});
    });
    await page.goto('http://clash.test/');
    await page.evaluate(raw => {
      window.host = clash3d.launch('#app');
      host.on('move', m => window.captureMove(m));
      host.emit('player',{index:0}); host.emit('preferences',{sound:false,mapView:'strategy',homeAtBottom:false}); host.emit('state',raw);
    },engine.stripSecret(JSON.stringify(state),0));
    await page.waitForTimeout(800);
    await page.getByRole('button',{name:'Move units and found cities',exact:true}).click();
    
    async function tap(position) {
      await page.locator(`.strategy-map-tile[data-position="${position}"]`).click();
      await page.waitForTimeout(200);
    }
    await tap('F7');

    await tap('F6');
    const assignments = page.locator('.boarding-assignment select');
    await assignments.first().waitFor();
    assert.equal(await assignments.count(),count);
    assert.equal(sent.length,0,'destination opens cargo choices without submitting');
    if (count === 2) {
      await assignments.nth(1).selectOption(String(pirates ? -4 : 11));
      assert.notEqual(await assignments.nth(0).inputValue(),await assignments.nth(1).inputValue());
    }
    await page.getByRole('region',{name:'Unit movement',exact:true}).getByRole('button',{name:/^Board ship/}).click();
    assert.equal(sent.length,1);
    const invalid = structuredClone(sent[0]);
    invalid.Movement.Move.boarding[0].units.push(invalid.Movement.Move.boarding[0].units[0]);
    assert.throws(() => engine.tryMove(JSON.stringify(state),JSON.stringify(invalid),0));
    const unknown = structuredClone(sent[0]);
    unknown.Movement.Move.boarding[0].embark_carrier_id = 9999;
    delete unknown.Movement.Move.boarding[0].embark_pirate;
    assert.throws(() => engine.tryMove(JSON.stringify(state),JSON.stringify(unknown),0));
    if (pirates) {
      const ordinary = structuredClone(state); ordinary.players[0].civilization = 'Rome';
      assert.throws(() => engine.tryMove(JSON.stringify(ordinary),JSON.stringify(sent[0]),0));
    }
    let after = JSON.parse(engine.tryMove(JSON.stringify(state),JSON.stringify(sent[0]),0));
    const ships = after.players[0].units.filter(u => u.unit_type === 'Ship');
    assert.deepEqual(ships.map(u => u.carried_units.length), count === 2 ? [1,1] : [2,2]);
    assert.equal(after.actions_left,2);
    assert.equal(after.state.Movement.movement_actions_left,2);
    assert.throws(() => engine.tryMove(JSON.stringify(after),JSON.stringify(sent[0]),0), 'no same-sea reassignment');
    after = JSON.parse(engine.tryMove(JSON.stringify(after),JSON.stringify({Movement:{Move:{units:ships.map(u=>u.id),destination:'G7'}}}),0));
    assert(after.players[0].units.every(u => u.position === 'G7'));
    assert.deepEqual(errors,[]);
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    console.log('Fleet boarding browser/WASM passed',{width,pirates,count});
    await page.close();
  }
} finally { await browser.close(); }
