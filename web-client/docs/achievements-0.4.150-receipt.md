# Clash of Cultures engine 0.4.150 delivery receipt

## Identity and reviewed source

- Game: **Clash of Cultures: Monumental edition**, BGS `clash/1`.
- Package: `@boardgamers/clash-server@0.4.150`, entry point `server.js`.
- Core approval: `e7928f088e02814d33c29cdf7d73dd1db28b5780`.
- Release/package approval and artifact source: `9ff814d17b3925c9b9e31fa3a8146fa18645dca1`.
- Fresh canonical fetch before delivery found no upstream drift. Source, protocol
  dependency/lockfile, release metadata and packaging recipe were non-force pushed
  to `origin/main`; the remote was verified to contain the reviewed source commit.
- This receipt is a documentation-only follow-up, not a new engine build.

## Exact reviewed and delivered artifact

Archive: `boardgamers-clash-server-0.4.150.tgz`

SHA-256:

```text
ce99ed5dc66c6104da6def723766bf7e84f5670bf318d74c5ef07e6f5f5b5d85
```

Hosted artifact:

<https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/engine/ce99ed5dc66c6104/boardgamers-clash-server-0.4.150.tgz>

Built from the committed release source with Rust 1.94.1, wasm-bindgen 0.2.105
and Cargo's locked dependency resolution. The scoped independent review approved
this exact archive after testing its unpacked Node/WASM engine. Package files were
checked against outputs from that build, and the package's dynamic-import named
exports and official protocol schemas were verified.

This is **same-build integrity, not a bit-reproducible-build claim**. A later review
rebuild changed ignored `.engine` outputs; wasm-bindgen invocations can produce
different bytes even with identical compiled input. Delivery did not rebuild or
repackage those outputs. The preserved approved tarball was hash-checked before
upload, and the hosted bytes were independently downloaded and verified against
the exact approved SHA-256. Manifest verification used the preserved tarball,
not the later ignored outputs.

## Activation and public verification

Installer activation verified at **2026-11-14T01:04:29.030Z**:

- Admin `GET /api/admin/gameinfo/clash/1` reports package 0.4.150 and the exact
  installer-owned `achievementManifest`: **revision 1, all 17 definitions**.
- The manifest was absent before upload and appeared after the installer processed
  this artifact; verification did not treat the upload response alone as activation.
- Public `GET /api/boardgame/clash/info/1` returned HTTP 200 and the exact manifest.
- Public album <https://boardgamers.space/badges/games/clash> returned HTTP 200,
  title `Clash of Cultures: Monumental edition achievements`, and all 17 names
  in its server-rendered page.
- Public counts endpoint returned HTTP 200 for all 17 IDs (all zero at verification).
- No fake games, test accounts, historical awards or live-game moves were created.

Only the raw reviewed tarball was sent to the existing engine-upload endpoint.
Credentials were read from `~/.bgs` in memory and were not committed or printed.
The POST response omits inherited gallery metadata; an initial comparison stopped
on that omission. A fresh full admin GET confirmed preservation, without reupload
or metadata writes. Subsequent installer verification compared the full GET again.

## Preserved scope

No viewer upload: hosted viewer **0.1.217** remains unchanged, including its URL:

<https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/viewer/assets-890e8364e4f3fe49236b/viewer.js>

Full before/after comparisons confirmed unchanged viewer, gallery, options,
preferences, settings, tutorial, meta, players and label. No practice-policy,
default-stack, database, production-configuration or game-rule changes were made.
The destroyed-wonder final-state caveat remains documented and regression-tested.

## Validation

- 105 Rust library tests passed with the pinned Rust 1.94.1 toolchain.
- Five achievement tests passed against WASM, including the independently reviewed
  unpacked archive; exact catalog, completed fixture, boundaries, losing-seat feats,
  serialization and secret filtering were checked.
- Svelte/TypeScript check: zero errors and zero warnings; locale checks passed.
- Prior core review independently passed 1,071 web tests.
- Strict Clippy diagnostics and the Zod advisory remain acknowledged baseline
  findings, not new release blockers. No unrelated remediation was bundled.
