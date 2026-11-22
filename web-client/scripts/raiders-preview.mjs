import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { raidersFixture } from './raiders-fixture.mjs';

export async function startPreview(port = 8645) {
  const engine = createRequire(import.meta.url)('../.engine/server.js');
  const raw = JSON.stringify(await raidersFixture(engine));
  // Never use a participant's secret-filtered state for the spectator preview.
  const publicState = engine.stripSecret(raw, undefined);
  const playerState = engine.stripSecret(raw, 1);
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://localhost');
    const path = url.pathname;
    if (path === '/') {
      const spectator = url.searchParams.get('seat') !== '1';
      const state = spectator ? publicState : playerState;
      res.setHeader('Content-Type', 'text/html');
      return res.end(`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Clash · synthetic Raiders preview</title><div id="app"></div><script src="/viewer.js"></script><script>
        window.host=clash3d.launch('#app');window.sent=[];window.prefs={sound:false};
        host.on('move',a=>sent.push(a));
        host.on('update:preference',({name,value})=>{prefs[name]=value;host.emit('preferences',prefs)});
        host.emit('player',${spectator ? '{}' : '{index:1}'});host.emit('theme',{dark:true});host.emit('preferences',prefs);
        host.emit('state',${JSON.stringify(state)});
      </script>`);
    }
    if (!['/viewer.js', '/server_bg.wasm'].includes(path)) { res.statusCode = 404; return res.end(); }
    res.setHeader('Content-Type', path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
    res.end(await readFile(new URL('../dist' + path, import.meta.url)));
  });
  await new Promise(resolve => server.listen(port, '127.0.0.1', resolve));
  return server;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await startPreview();
  console.log('Synthetic Raiders preview: http://localhost:8645/ (normal viewer, no saved moves)');
}
