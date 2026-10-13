import {test} from 'node:test';
import assert from 'node:assert/strict';
import {regionEdges} from './region-overlay.ts';
test('region perimeters remove internal tile boundaries',()=>{
  assert.equal(regionEdges(['A1']).length,6);
  assert.equal(regionEdges(['A1','B1']).length,10);
  assert.equal(regionEdges(['A1','A2']).length,10);
});

import {createRequire} from 'node:module';
const engine = createRequire(import.meta.url)('../.engine/server.js');
test('public regions partition each supported map and identify all starting regions', async()=>{
  for (const count of [2,3,4]) {
    const raw=await engine.init(count,[],{},'regions',{});
    const game=JSON.parse(engine.stripSecret(raw,0));
    const view=JSON.parse(engine.webView(JSON.stringify(game),0));
    assert.equal(view.regions.filter((r:any)=>r.homePlayer !== null).length,count);
    const tiles=view.regions.flatMap((r:any)=>r.tiles);
    assert.equal(new Set(tiles).size,tiles.length);
    assert.deepEqual([...tiles].sort(), game.map.tiles.map(([p]:[string,unknown])=>p).sort());
  }
});
