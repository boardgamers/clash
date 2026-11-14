# Clash achievement artwork — review evidence

Issue: https://codeberg.org/boardgamers/boardgamers/issues/595

Canonical GitHub origin/main fetched and fast-forward integration confirmed at
`0099fa11cd697080bc8dccda629d8d4f5c92aae5`. Engine 0.4.151 → 0.4.152;
achievement revision 2 → 3. Protocol remains 0.11. No publication performed.

## Exhaustive coverage


Live gameinfo v1; engine 0.4.151; manifest revision 2.

| Exact achievement ID | Current icon | Decision / semantic design |
|---|---|---|
| `complete-shorter` | Omitted → game fallback | Four-age city frieze with completion seal, not a winner crown. |
| `win-shorter` | Omitted → game fallback | Four-age city frieze with prominent gold victory crown, distinct from completion variant. |
| `complete-standard` | Omitted → game fallback | Six-age city frieze with completion seal, not a winner crown. |
| `win-standard` | Omitted → game fallback | Six-age city frieze with prominent gold victory crown, distinct from completion variant. |
| `complete-roll-to-end` | Omitted → game fallback | Age procession ending at rolled die with completion seal, not a winner crown. |
| `win-roll-to-end` | Omitted → game fallback | Age procession ending at rolled die with prominent gold victory crown, distinct from completion variant. |
| `complete-wonderful-ending` | Omitted → game fallback | Age procession ending at completed monument with completion seal, not a winner crown. |
| `win-wonderful-ending` | Omitted → game fallback | Age procession ending at completed monument with prominent gold victory crown, distinct from completion variant. |
| `complete-epic` | Omitted → game fallback | Extended eight-age civilization panorama with completion seal, not a winner crown. |
| `win-epic` | Omitted → game fallback | Extended eight-age civilization panorama with prominent gold victory crown, distinct from completion variant. |
| `score-30` | Omitted → game fallback | Civilization scoring panorama combining city, knowledge and monument; secondary 30. |
| `score-40` | Omitted → game fallback | Higher-relief expanded civilization panorama, secondary 40; structural progression beyond numeral. |
| `seven-cities` | Omitted → game fallback | Seven city groups spread over connected territory. |
| `twenty-advances` | Omitted → game fallback | Branching regular-advance research tableau, secondary 20; no special-advance substitution. |
| `three-objectives` | Omitted → game fallback | Three different objective tablets bearing completion seals. |
| `two-wonders-built` | Omitted → game fallback | Two distinct wonders rising from construction foundations; not captured monuments. |
| `captured-leader` | Omitted → game fallback | Leader bust or standard detained in enclosure, not a dead leader. |
| `built-great-arena` | Omitted → game fallback | Oval arena with tiered seating. |
| `built-great-pyramids` | Omitted → game fallback | Architectural pyramid group, not generic mountains. |
| `built-great-gardens` | Omitted → game fallback | Terraced architectural gardens with cascading vegetation. |
| `built-great-library` | Omitted → game fallback | Monumental library with shelves/tablets integrated into facade. |
| `built-great-lighthouse` | Omitted → game fallback | Coastal lighthouse tower casting beam. |
| `built-great-mausoleum` | Omitted → game fallback | Tiered monumental tomb. |
| `built-great-statue` | Omitted → game fallback | Monumental standing statue on substantial plinth. |
| `built-great-wall` | Omitted → game fallback | Crenellated wall traversing terrain. |
| `complete-builder` | Omitted → game fallback | Peaceful construction scene with tools and growing city, no military victory claim. |
| `win-rome` | achievement-faction-rome | PRESERVE existing achievement-faction-rome unchanged. |
| `win-greece` | achievement-faction-greece | PRESERVE existing achievement-faction-greece unchanged. |
| `win-china` | achievement-faction-china | PRESERVE existing achievement-faction-china unchanged. |
| `win-vikings` | achievement-faction-vikings | PRESERVE existing achievement-faction-vikings unchanged. |
| `win-babylonia` | achievement-faction-babylonia | PRESERVE existing achievement-faction-babylonia unchanged. |
| `win-india` | achievement-faction-india | PRESERVE existing achievement-faction-india unchanged. |
| `win-egypt` | achievement-faction-egypt | PRESERVE existing achievement-faction-egypt unchanged. |
| `win-phoenicia` | achievement-faction-phoenicia | PRESERVE existing achievement-faction-phoenicia unchanged. |
| `win-maya` | achievement-faction-maya | PRESERVE existing achievement-faction-maya unchanged. |
| `win-persia` | achievement-faction-persia | PRESERVE existing achievement-faction-persia unchanged. |
| `win-japan` | achievement-faction-japan | PRESERVE existing achievement-faction-japan unchanged. |
| `win-huns` | achievement-faction-huns | PRESERVE existing achievement-faction-huns unchanged. |
| `win-celts` | achievement-faction-celts | PRESERVE existing achievement-faction-celts unchanged. |
| `win-aztecs` | achievement-faction-aztecs | PRESERVE existing achievement-faction-aztecs unchanged. |
| `win-carthage` | achievement-faction-carthage | PRESERVE existing achievement-faction-carthage unchanged. |


## Implementation and provenance

The 26 new keys are `achievement-<exact ID>`. Original editable procedural SVG
geometry lives in `scripts/nonfaction-art.mjs` under the repository MIT licence.
No external image, wonder PNG, photograph, traced artwork, or uncertain licence
is reused. Generic system sans-serif renders only supplementary score numerals.
Four/six/eight city marks denote ages, not a city-award count; seven-cities has
exactly seven house groups, twenty-advances has exactly twenty research nodes,
and three-objectives has exactly three completed tablets. Wonders use oval
arches, pyramid faces, planted terraces, shelved facade, coastal light tower,
tiered tomb, standing figure and crenellated terrain respectively. Victory crowns
are used only for wins; completion uses a check. Builder depicts a crane and house.

The 15 faction keys, legacy manifest, inputs validator, publisher and underlying
source inputs are untouched. They are NOT regenerated or included in the new
26-asset output directory/envelope. Prior public URL/full hashes are pinned in
the local publication envelope; fresh public verification is still required at
release time. Existing licences are not newly asserted or broadened.

## Reproduction and checks

From web-client: `npm ci --ignore-scripts`, then
`node scripts/build-achievement-assets.mjs --nonfaction`.
`--nonfaction --check` validates source hash, output hash, exact key coverage,
unique outputs and dimensions. This branch performs no uploads and requires no
HTTP service: Chromium renders inline original SVG and PNG data URLs.

`cargo test -p server --lib achievements --locked`: 14 passed.
`bash web-client/scripts/build-bridge.sh`: release WASM build succeeded.
`node scripts/package-engine.mjs`: packs exactly the built bridge.
`node --test scripts/nonfaction-assets.test.mjs`: predicate byte comparison,
legacy preservation, packed 41-row manifest, protocol hooks, image decoding.
`npm run test:achievement-assets`: legacy guard regression suite.

## Review artifacts and release gate

Under `web-client/.engine/achievement-assets-nonfaction/`:
26 editable SVG exports, 26 PNGs at 256px, receipt.json, contact-sheet.html,
contact-sheet.png (32/48/64 circular display on light/dark), publication-envelope.json.
Manifest: `web-client/release/achievement-assets-nonfaction.json`.

Exact tested package: `.engine/releases/boardgamers-clash-server-0.4.152.tgz`
SHA256 `61a7f537d547cd0d4baea74f5d1331ffca0804779dcdb59bb566742fc59427db`.
The .engine artifacts are intentionally untracked; retain this worktree.

The old faction publisher remains intact. Use the coordinator's narrow asset-only
publisher with this exact 26-key envelope AFTER fresh independent code and actual
visual review approval. No credentials have been read. No asset PUT, engine POST,
activation, metadata changes or source push has been made. Engine POST immediately
activates: only do so after all 26 hosted images are fetched anonymously, hash
verified and decoded and preservation of all 15 old assets is rechecked.

Release source target: nonforce canonical GitHub origin/main. Coordination issue
is on Codeberg; no Codeberg git mirror is configured in this worktree. Refresh
origin/main and re-review/retest concurrent changes before release. Commit source
and sanitized publication receipts together in the approved publication task.
