# clash

![Lines of code](https://img.shields.io/tokei/lines/github/boardgamers/clash)

## Client

### Run native client

- `cd client`
- `cargo run`.

### Run local web client

- `cd client`
- `./wasm-bindgen-macroquad.sh local_client`
  In the `client` directory, run `basic-http-server .` in `dist/` and open `http://localhost:4000` in a browser.

### Server wrapper

- `cd server`
- `cargo install wasm-pack` (if you haven't already)
- `./build-wasm.sh`

# Notes

- https://stackoverflow.com/questions/40102686/how-to-install-package-with-local-path-by-yarn-it-couldnt-find-package

## Boardgamers Mono

- `docker run -d -p 27017:27017 mongo:4.4`
- `cd apps/api && pnpm seed && echo cron=1 > .env`
- `pnpm dev --filter @bgs/api --filter @bgs/game-server --filter @bgs/web --filter @bgs/admin`
- admin: http://localhost:3000 (admin@test.com/password)
- user: http://localhost:8612/ (user@test.com/password)

old

- https://github.com/boardgamers/boardgamers-mono/blob/683f4d473586ffe359ad7e58f7bf08d95c96d821/.gitpod.yml#L12-L18 (if
  you have't already)
    - This will create three users, with emails admin@test.com, user@test.com and user2@test.com, and "password"
      password".

.tool-versions

```
nodejs 14.21.3
pnpm 6.14.1
```

### Bypass npm publish

```
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

## Docs

- [Game API](https://docs.boardgamers.space/guide/engine-api.html)
