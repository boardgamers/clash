# clash

![Lines of code](https://img.shields.io/tokei/lines/github/boardgamers/clash)

## Client

### Run native client

- `cd client`
- `cargo run`.

### Run remote with boardgamers-mono web client locally

- `./scripts/build-remote-client.sh`
- `cd client/js/dist && basic-http-server .`
- `google-chrome --js-flags='--stack-trace-limit 100' --disable-web-security --user-data-dir=/tmp http://localhost:8612`

### Server wrapper

- `cd server`
- `cargo install wasm-pack` (if you haven't already)
- `./build-remote-server.sh`

# Notes

- https://stackoverflow.com/questions/40102686/how-to-install-package-with-local-path-by-yarn-it-couldnt-find-package
- class:d-none={!stateSent}

# Todo

- Add https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js to scripts

## Boardgamers Mono

- `docker run -d -p 27017:27017 mongo:4.4`
- `cd apps/api && pnpm seed && echo cron=1 > .env`
- `pnpm dev --filter @bgs/api --filter @bgs/game-server --filter @bgs/web --filter @bgs/admin`
- admin: http://localhost:3000 (admin@test.com / password)
- user: http://localhost:8612/ (user@test.com / password)

old

- https://github.com/boardgamers/boardgamers-mono/blob/683f4d473586ffe359ad7e58f7bf08d95c96d821/.gitpod.yml#L12-L18 (if
  you haven't already)
    - This will create three users, with emails admin@test.com, user@test.com and user2@test.com, and "password"
      password".

.tool-versions

```
nodejs 16.x
pnpm 6.14.1
```

### Bypass npm publish

```diff
Index: apps/game-server/app/services/engines.ts
IDEA additional info:
Subsystem: com.intellij.openapi.diff.impl.patch.CharsetEP
<+>UTF-8
===================================================================
diff --git a/apps/game-server/app/services/engines.ts b/apps/game-server/app/services/engines.ts
--- a/apps/game-server/app/services/engines.ts	(revision 741eecd403ed7c5b38b3b98b1e26be8a502cafc0)
+++ b/apps/game-server/app/services/engines.ts	(date 1726905060117)
@@ -7,9 +7,7 @@
 const engines = {};
 
 async function requirePath(name: string, version: number) {
-  const entryPoint = (await GameInfo.findById({ game: name, version }, "engine.entryPoint", { lean: true })).engine
-    .entryPoint;
-  return `../../games/node_modules/${name}_${version}/${entryPoint}`;
+  return `/home/gregor/source/clash/server/pkg`;
 }
 
 export async function getEngine(name: string, version: number): Promise<Engine> {
```

### Set viewer directly

```diff
Index: apps/api/app/resources.ts
IDEA additional info:
Subsystem: com.intellij.openapi.diff.impl.patch.CharsetEP
<+>UTF-8
===================================================================
diff --git a/apps/api/app/resources.ts b/apps/api/app/resources.ts
--- a/apps/api/app/resources.ts	(revision 741eecd403ed7c5b38b3b98b1e26be8a502cafc0)
+++ b/apps/api/app/resources.ts	(date 1726922668897)
@@ -31,7 +31,8 @@
 
   const viewer: ViewerInfo =
     gameInfo?.viewer?.alternate?.url && ctx.query.alternate === "1" ? gameInfo?.viewer.alternate : gameInfo.viewer;
-  const viewerUrl = ctx.query.customViewerUrl || viewer.url;
+  const viewerUrl = "http://127.0.0.1:4000/client.js";
 
   ctx.body = `
     <html>
@@ -48,7 +49,9 @@
         </div>
       </body>
       <${"script"} type='text/javascript'>
-        const gameObj = window.${viewer.topLevelVariable}.launch('#app');
+        const gameObj = window.clash.launch('#app');
         window.addEventListener('message', event => {
           console.log('received message from controller', event.data.type, JSON.parse(JSON.stringify(event.data)));
           switch (event.data.type) {
```
     
GameInfo.json (for now)

```json
  {
    "_id": {
      "game": "clash",
      "version": 1
    },
    "description": "Clash of Cultures",
    "engine": {
      "package": {
        "name": "@boardgamers/clash-server",
        "version": "*"
      },
      "entryPoint": "server.js"
    },
    "expansions": [],
    "label": "Clash of Cultures",
    "meta": {
      "public": true,
      "needOwnership": true
    },
    "options": [],
    "players": [
      2,
      3,
      4
    ],
    "preferences": [],
    "rules": "",
    "viewer": {
      "url": "//cdn.jsdelivr.net/npm/@boardgamers/clash-client@0.0.5/client.js",
      "topLevelVariable": "clash",
      "dependencies": {
        "scripts": [],
        "stylesheets": []
      },
      "replayable": true,
      "alternate": null,
      "settings": []
    }
  }
```


## Docs

- [Game API](https://docs.boardgamers.space/guide/engine-api.html)
