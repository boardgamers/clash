# Clash engine 0.4.152 revision-3 source delivery receipt

Independent anonymous GET verification completed 2026-11-14T13:00:09Z for
<https://boardgamers.space/api/boardgame/clash/info/latest>.

## Approved source and artifact

- Source: `384bcbe254933ad4ee5369c07aacfd87c51d15e8`.
- Independent artwork/code reviewer approval: `sub-1794658501110-61ce915d`
  (shared issue-595 coordination record).
- Package: `@boardgamers/clash-server@0.4.152`, manifest revision **3**,
  **41** achievements: **26** new original nonfaction images and **15** preserved
  faction images.
- Exact original archive SHA-256:
  `61a7f537d547cd0d4baea74f5d1331ffca0804779dcdb59bb566742fc59427db`.
  Both the existing local archive and anonymously downloaded hosted archive match.
- [Sanitized combined activation receipt](sanitized-activation-receipts/clash.json)
  records full package, asset, source-image, envelope and activation evidence hashes.
  Private receipt hashes are integrity references only; no private receipts,
  metadata snapshots, credentials or authorization headers are included.

## Verification and scope

The public engine package and full achievement manifest exactly matched the
receipt. All **58** public asset references and downloaded SHA-256/size pairs
matched: 26 new achievement images, 15 preserved faction images, and 17 other
preserved assets. Existing faction artwork and evaluation logic remain unchanged.

Focused checks:

- `cargo test -p server --lib achievements`: 14 passed.
- `node --test scripts/nonfaction-assets.test.mjs scripts/achievement-assets.test.mjs scripts/publish-achievement-assets.test.mjs scripts/publish-achievement-backups.test.mjs`
  from `web-client`: 22 passed, 2 skipped, 0 failed. Includes packed-engine schema,
  original faction/evaluator identity, and decoding all 26 generated images.
- Sanitized JSON key allowlist inspection and full public GET/hash verification passed.

This source-delivery operation performed no uploads, engine POSTs, repacking or
artifact builds. Viewer, settings and visibility were not edited; the central
publisher's combined receipt records unchanged metadata verification. No private
metadata is copied here.

Canonical release remote is `git@github.com:boardgamers/clash.git`, branch `main`;
no additional public mirror is configured. Fresh fetch found `main` at
`0099fa11cd697080bc8dccda629d8d4f5c92aae5`, an ancestor of approved source, with no
concurrent changes requiring integration. Delivery uses a nonforce fast-forward
push of approved source and this documentation-only receipt from the retained
`moon/achievement-art-2026` worktree. The original checkout remains untouched.
