# Achievement emblem artwork — local review only

The adjacent `../achievement-assets.json` is the tracked input manifest (revision
2). It binds the 15 playable registry names to the actual canonical emblems in
`src/civilization-emblems.ts`: four PNG masks and eleven Lucide components. NPCs
and the civilization-selection placeholder are excluded. Unknown names never
receive the UI's Landmark fallback. No substitute logos or SVG path copies are
maintained here.

## Generate and verify

From the repository root, using the existing installed web-client dependencies
and Playwright Chromium:

```sh
node web-client/scripts/build-achievement-assets.mjs
node web-client/scripts/build-achievement-assets.mjs --check
node --test web-client/scripts/achievement-assets.test.mjs
```

The builder starts an isolated ephemeral loopback Vite server (not the app's
production configuration), blocks off-origin browser requests, and closes both
server and browser in `finally`. It mounts actual Svelte components with the map
flag's `strokeWidth: 2.5`, serializes them to SVG, then uses the same alpha-mask /
`source-in` coloring technique as `civilization-flags.ts` for both source types.
Each symbol is centered in a 192px box on a 256×256 opaque neutral light-gray
background with dark ink. Standalone badges contain no text. The opaque neutral
background keeps the symbol readable against either light or dark album chrome.

All generated files are ignored under `web-client/.engine/achievement-assets/`:

- `achievement-faction-<lowercase registry name>.png` (15 files)
- `receipt.json`: revision 2, manifest SHA-256, dimensions, Chromium/platform
  metadata, and `assets` entries containing `civilization`, `key`, basename
  `file`, `sha256`, `width`, `height`
- `contact-sheet.html` and `contact-sheet.png`: labeled independent review sheets
  showing all symbols on light and dark surrounds. Labels are not in badge PNGs.

The builder verifies two independent page renders are byte-identical before
writing output. Input hashes, Lucide 0.577.0, dimensions, mapping, and generated
hashes are checked locally. Chromium/platform metadata is recorded because
cross-platform/browser-version rasterization is not claimed byte-identical.
To reproduce release bytes use the same lockfile and recorded Chromium/platform.
The manifest is intentionally not automatically refreshed when canonical sources
change: inspect the changes, review provenance, and deliberately update hashes.
A successful local build is **not** artwork approval or permission to publish.
No uploads are performed by these tools.

## Attribution and established project reuse

The eleven Lucide symbols come from `lucide-svelte` **0.577.0**,
https://github.com/lucide-icons/lucide. The full package license is copied verbatim
to `LICENSE-lucide.txt`: ISC, with the included MIT notice for Feather-derived
portions. Preserve these notices with distributions of the derived artwork.

Rome, Greece, China, and Vikings reuse the repository's original PNG assets in
`client/assets/` (exact original paths and SHA-256 hashes are in the manifest).
The user explicitly authorized reuse of this established project artwork after
confirmation that the original import bytes and currently served viewer PNGs match.
The manifest records source paths/hashes, historical commits and production-use
evidence as `verified-existing-project-asset`. The publisher accepts only these four
pinned identities through this path; it is not an arbitrary missing-license bypass.
Per-item licenses remain **unverified** (`license: null`); no CC0 or blanket root-MIT
coverage is claimed. No known restriction was found in the bounded investigation.
Independent review of the final delta and exact artwork receipt remains required.

## Durable publication backups

Before any PUT, the publisher saves original metadata and version snapshots under
`.engine/releases/achievement-assets/run-*`. Each run has a unique 0700 directory,
exclusive-created 0600 snapshots, and fsynced files/directories. Failure to persist
backups or the initial receipt stops uploads. Retries never overwrite prior runs.
The per-run `upload-receipt.json` references snapshot paths and SHA-256 hashes;
credentials/request headers are not included. Preserve these records if an upload
fails. Recovery is manual and limited to affected named assets after inspection;
this tool deliberately provides no broad metadata restore operation.
