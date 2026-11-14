# Clash of Cultures engine 0.4.151 delivery receipt

Verified live at **2026-11-14T01:39:56.246Z**, BGS `clash/1`.

## Reviewed source and canonical delivery

- Core revision-2 source/artwork: `61be734999295a87a2fb30cdcc8c3ea66595c365`.
- Final approved provenance/backup delta: `1684b3064d7d9d80e69af6593e11ed636abf7670`.
- Fresh canonical fetch found no upstream drift. Both commits were pushed without
  force to `git@github.com:boardgamers/clash.git`, `main`, before production uploads.
  A fresh fetch confirmed the reviewed source was contained in the release branch.
- This receipt is documentation only; no engine rebuild or repackaging occurred.
- GitHub reported authorized bypass of the required-PR/status-check rules during
  the main push. No force push was used. No separate public mirror is configured.

## Exact preserved engine artifact

`@boardgamers/clash-server@0.4.151`, entry point `server.js`:

```text
ec0f936cf29a908835f98048f885ff5b74f0c8ec5654122b218864ce2846e7bd
```

Hosted archive:
<https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/engine/ec0f936cf29a9088/boardgamers-clash-server-0.4.151.tgz>

The approved archive was hash-checked, uploaded unchanged through the engine-only
endpoint, and anonymously downloaded to verify its full SHA-256. Activation was
confirmed from the installer-owned manifest, not merely the upload response.
This is same-build integrity, not a claim of bit-reproducible WASM rebuilding.

## Artwork and durable backups

Approved local artwork receipt SHA-256:

```text
0830bc330eca0e072e2b61abdd798b26d933a3dd26593198d0ecd156741cf021
```

All 15 PNGs retained their reviewed image bytes. The manifest records the explicit
user-authorized reuse of four established project symbols, with pinned historical
and existing-production identity. Their per-item licenses remain unverified;
no CC0 or blanket MIT coverage is asserted. Lucide notices remain preserved.

Before the first asset PUT, the publisher persisted the original full metadata and
version snapshots in the ignored directory:

`web-client/.engine/releases/achievement-assets/run-vtdR9K/`

- Directory mode: 0700; exclusive snapshot files: 0600; files/directories fsynced.
- `meta.json`: `8f5a302baff12bf10dcbc9d87ca7b822b12b4192c0a2bbeb5c3f61742abccfbc`
- `version.json`: `477925737560e86679467e1c78ccbc45b243b76f74280a23c54721a4cbefb0d9`
- The private per-run `upload-receipt.json` references these originals. They remain
  local, outside generated artwork output; retries cannot overwrite them.
- [Public delivery evidence](achievements-0.4.151-public.json) records all 15 hosted
  asset URLs, byte lengths, full downloaded WebP hashes and source PNG hashes.
  It contains no credentials or private metadata snapshots.

The helper verified all named assets with anonymous, redirect-disabled HTTPS GETs,
including WebP type/magic, byte lengths and URL hash prefixes. All 15 public asset
hashes were checked again before engine upload. Full metadata/version comparisons
confirmed unrelated assets and gallery were preserved. No broad restore was used.

## Installed catalog and public presentation

- Full admin GET and public `GET /api/boardgame/clash/info/1` both exactly match the
  preserved archive's **revision 2, 41-definition** catalog.
- Includes `seven-cities`, all 15 faction wins, all eight individual wonder feats,
  `complete-builder`, and Standard-only wording. The retired `four-cities` is absent.
- <https://boardgamers.space/badges/games/clash> returned HTTP 200 with all 41 names
  and all 15 expected durable faction image URLs in its server-rendered page.
- Full before/after version comparisons differed only in `engine`,
  `achievementManifest`, and server-maintained `updatedAt` after asset staging.
- Viewer remains **0.1.217**, with its complete configuration and URL unchanged:
  <https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/viewer/assets-890e8364e4f3fe49236b/viewer.js>
- No viewer upload, fake production games/accounts, live-game moves, historical
  backfill, policy/configuration changes, or database edits were performed.

## Validation

- 111 Rust library tests; 1,076 web tests; Svelte check with zero errors/warnings;
  locale checks passed during preparation.
- Independent exact-archive verification passed all 10 achievement tests.
- Final provenance/publisher/backup/artwork suite: **21 passed, zero skipped**,
  rerun immediately before delivery. Covers restricted backup permissions,
  unique retries, ordering/failure before PUT, provenance identity tampering,
  public URL/hash verification and credential-free receipts/errors.
- Existing broad Clippy warnings were not used as a new release gate.
