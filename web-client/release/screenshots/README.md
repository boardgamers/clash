# Release gallery — real completed game

Local preview assets, **not published** by the capture script.

## Provenance

- Public source: https://boardgamers.space/game/Open-clash-a52a21ee
- Completed three-player game: Greece, China, Celts; age VI, finished.
- Actual final 60-tile map, with 5 / 4 / 7 player cities (plus barbarians).
- Geometry, terrain, cities, buildings, units, public advances, scores and game outcome are unchanged. No tutorial position, invented city cluster, or reshaped map is used.
- Exact capture date, viewer version and SHA-256 of map + city/unit positions are in `checks.json`. The script pins that hash and refuses to silently capture a changed board.

## Reproduce

From the repository root, with the existing built viewer and engine bridge available:

```sh
node web-client/scripts/release-gallery.mjs
```

Uses an **anonymous GET** of the completed game's public endpoint. No credentials are read, and there are no live writes, moves or history changes. The viewer runs offline against intercepted local `dist` assets; all other browser requests are blocked. Capture uses the existing build (viewer 0.1.211), not a newly bumped release.

The public response is not a safe fixture: completed-game responses can contain hands. The script immediately passes it through `engine.stripSecret(data, undefined)` (the engine's spectator index), then removes player names, messages, log and replay history from the in-memory rendering copy. It does not save game JSON, shuffled decks, seeds, hands, usernames or free-text history. Names displayed in the images are civilization names. Only screenshots and minimal checks/provenance are retained.

Board/strategy/mobile views remain spectators. For the research screenshot only, the local host selects Greece's viewing seat because the Research button is disabled for an unselected spectator. **The same spectator-filtered payload is retained**; no player-specific state is fetched, and no hand is revealed. Returning to the board restores the spectator seat. This is an inspection of real public advances, not a made-up research choice or game move.

The desktop camera uses the viewer's actual map framing. Mobile uses an actual right-drag pan to move the complete strategy map below the three stacked player cards; no geometry or CSS is altered. Captures wait for player cards and loading completion. Checks require no page errors, no emitted moves, no mobile horizontal overflow, and record host protocol diagnostics.

## Assets and review

- `board-{light,dark}.png`: 1440 × 960, full actual 3D map.
- `research-{light,dark}.png`: 1440 × 960, Greece's real public research.
- `strategy-{light,dark}.png`: 1440 × 960, full actual strategy map.
- `mobile-{light,dark}.png`: 390 × 844, full strategy map below player cards.

All eight images were visually reviewed in both themes. These PNGs are tracked repository assets, not ignored build output.

The existing publication gallery uses **board / research / mobile** pairs; desktop strategy images are supplemental previews. **Do not run `publish-release-content.mjs` to publish just these images:** it also replaces 16 rules pages and metadata. The approved release uses `node web-client/scripts/publish-gallery.mjs --publish` after the matching `main` commit passes CI. Omit `--publish` for a read-only plan; `--check` validates the eight frozen captures without credentials or network. Run `node --test web-client/scripts/publish-gallery.test.mjs` for preservation tests.

The gallery-only publisher uploads six content-derived asset keys from `manifest.json`, verifies each anonymous hosted download, then switches the gallery in one metadata operation. Existing captions, order, unrelated entries and old assets are preserved. It verifies all 16 published rules pages against `release/rules.json` without writing them, and asserts engine, viewer, options and other metadata are unchanged. Evidence defaults to `/tmp/gallery-real-games-release` and contains only public release metadata and image hashes/downloads.

**Platform format limitation:** BGS always converts uploaded images to WebP (screenshot pipeline: rotate, fit inside 2400 × 2400 without enlargement, WebP quality 88). Hosting byte-identical original PNGs through the supported asset API is not possible. `manifest.json` therefore pins both the approved original PNG hashes and the expected converted WebP hashes, generated using the recorded Sharp/libvips versions and that exact pipeline. The publisher compares downloaded WebP bytes against those conversion hashes before changing the gallery. Original PNGs remain frozen in Git; no viewer/engine publication or version bump is needed.
