import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';
const require = createRequire(import.meta.url);
export function previewApi(): Plugin {
  return {
    name: 'clash-local-preview',
    configureServer(server) {
      const engine = require(fileURLToPath(new URL('../.engine/server.js', import.meta.url)));
      const save =
        process.env.CLASH_PREVIEW_SAVE ??
        fileURLToPath(new URL('../.engine/preview-state.json', import.meta.url));
      let state: string;
      let revision = 0;
      let messages: any[] = [];
      let readAt: Record<string, number> = {};
      const persist = () => writeFileSync(save, JSON.stringify({ state, revision, messages, readAt }));
      const reset = async () => {
        state = await engine.init(2, [], { civilization: 'Random' }, 'clash-preview-20260927', {});
        for (let i = 0; i < 2; i++)
          state = engine.setPlayerMetaData(state, i, { name: ['Leif', 'Aurelia'][i] });
        revision++;
        messages = [];
        readAt = {};
        persist();
      };
      const ready = (async () => {
        if (existsSync(save)) {
          const saved = JSON.parse(readFileSync(save, 'utf8'));
          state = saved.state;
          revision = saved.revision;
          messages = saved.messages;
          readAt = saved.readAt ?? {};
          engine.webView(state, engine.currentPlayer(state));
        } else await reset();
      })();
      server.middlewares.use(async (req, res, next) => {
        const url = new URL(req.url ?? '/', 'http://127.0.0.1:8643');
        if (url.pathname.startsWith('/bundle/')) {
          const asset = url.pathname.slice(8);
          if (
            !/^[a-zA-Z0-9_-]+\.(?:js|wasm|json|woff2)$/.test(asset) &&
            !['engine/server.js', 'engine/server_bg.wasm'].includes(asset)
          ) {
            res.statusCode = 404;
            res.end();
            return;
          }
          try {
            res.setHeader(
              'content-type',
              asset.endsWith('.wasm')
                ? 'application/wasm'
                : asset.endsWith('.json')
                  ? 'application/json'
                  : asset.endsWith('.woff2')
                    ? 'font/woff2'
                    : 'text/javascript',
            );
            res.end(readFileSync(fileURLToPath(new URL('../dist/' + asset, import.meta.url))));
          } catch {
            res.statusCode = 404;
            res.end('Build the viewer first');
          }
          return;
        }
        if (!url.pathname.startsWith('/api/') && !url.pathname.startsWith('/legacy/')) return next();
        try {
          if (url.pathname.startsWith('/legacy/')) {
            const base =
              'https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/viewer/locales-b2602be75e79cd1b/';
            const response = await fetch(new URL(url.pathname.slice(8), base));
            res.writeHead(response.status, {
              'content-type': response.headers.get('content-type') ?? 'application/octet-stream',
            });
            res.end(Buffer.from(await response.arrayBuffer()));
            return;
          }
          await ready;
          res.setHeader('content-type', 'application/json');
          res.setHeader('cache-control', 'no-store');
          let body: any = {};
          if (req.method === 'POST') {
            if (req.headers.origin && req.headers.origin !== `http://${req.headers.host}`) {
              res.writeHead(403).end('{}');
              return;
            }
            let text = '';
            for await (const chunk of req) {
              text += chunk;
              if (text.length > 100000) throw Error('Request too large');
            }
            body = JSON.parse(text || '{}');
          }
          if (url.pathname === '/api/reset' && req.method === 'POST') await reset();
          if (url.pathname === '/api/settings' && req.method === 'POST') {
            if (body.seat !== 0 && body.seat !== 1) throw Error('Choose a player.');
            if (body.name !== 'skipRazeCity' || typeof body.value !== 'boolean')
              throw Error('Invalid setting.');
            state = engine.setPlayerSettings(state, body.seat, { [body.name]: body.value });
            persist();
            res.end(JSON.stringify({ settings: engine.playerSettings(state, body.seat) }));
            return;
          }
          if (url.pathname === '/api/move' && req.method === 'POST') {
            if (body.revision !== revision) throw Error('The game changed. Please choose your action again.');
            if (!Number.isInteger(body.seat) || body.seat !== engine.currentPlayer(state))
              throw Error('It is not your turn.');
            state = engine.tryMove(
              state,
              typeof body.move === 'string' ? body.move : JSON.stringify(body.move),
              body.seat,
            );
            revision++;
            persist();
          }
          if (url.pathname === '/api/chat' && req.method === 'POST') {
            if (body.seat !== 0 && body.seat !== 1)
              throw Error('Spectators cannot send messages in this preview.');
            if (typeof body.text !== 'string' || !body.text.trim() || body.text.length > 5000)
              throw Error('Enter a message under 5000 characters.');
            const createdAt = new Date().toISOString();
            messages.push({
              _id: crypto.randomUUID().replaceAll('-', '').slice(0, 24),
              text: body.text.trim(),
              author: ['Leif', 'Aurelia'][body.seat],
              authorId: `preview-${body.seat}`,
              playerIndex: body.seat,
              createdAt,
              type: 'text',
            });
            persist();
          }
          if (url.pathname === '/api/read' && req.method === 'POST') {
            if (body.seat !== 0 && body.seat !== 1) throw Error('Unknown player');
            const message = messages.find((m) => m._id === body.messageId);
            if (message) readAt[body.seat] = Math.max(readAt[body.seat] ?? 0, Date.parse(message.createdAt));
            persist();
          }
          const active = engine.currentPlayer(state);
          const requested = url.searchParams.get('seat');
          const seat =
            requested === 'spectator' ? undefined : requested === '0' ? 0 : requested === '1' ? 1 : active;
          res.end(
            JSON.stringify({
              state: engine.stripSecret(state, seat),
              settings: seat === undefined ? null : engine.playerSettings(state, seat),
              seat,
              revision,
              messages,
              lastReadAt: readAt[String(seat)] ?? 0,
            }),
          );
        } catch (error) {
          res.statusCode = 400;
          res.end(JSON.stringify({ error: String(error) }));
        }
      });
    },
  };
}
