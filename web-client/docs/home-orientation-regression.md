# Home-at-bottom host preferences regression

The reported production failure was not camera math: platform badge SVGs were
inlined as `data:` URLs. The protocol requires absolute HTTP(S) badge URLs, so a
single invalid badge rejected the **entire** preferences event, including
`homeAtBottom`. The platform fix emits SVG assets with HTTP(S) URLs; Clash must
not weaken URL validation to accept unsafe or relative URLs.

`src/host-preferences.test.ts` covers rejection of the full snapshot at the
reported badge paths, successful delivery of the complete synthetic host
preferences, and continued rejection of unsafe URLs. It runs in `npm test`.

For rendered-camera coverage, install Playwright Chromium, prepare the engine
bridge (`npm run build:bridge`), and run:

```sh
npm run test:home-orientation
```

This builds the viewer and checks every seat in 2-, 3-, and 4-player games in
both 3D and strategy modes with the full synthetic host preferences. It checks
callback ordering, state refresh, toggles, resize, fullscreen, side panels,
and manual camera orbit/reset. The rejected inline-badge snapshot is reproduced
through the real viewer emitter before sending valid preferences.

An optional anonymous public gameplay response can be exercised locally:

```sh
node scripts/home-orientation-smoke.mjs --snapshot=/tmp/public-game.json --seat=1
```

The script requires `secrets_stripped: true` in the game state. Keep downloaded
snapshots outside the repository. This is an offline viewer harness: it does not
submit live game moves. Synthetic preferences avoid capturing account metadata.

These are regression tests only: no engine/viewer version bump or BGS publication
is required. Passing them does not establish that the separate platform fix has
been deployed.
