# Publishing the viewer

Upload both `web-client/dist/viewer.js` and `web-client/dist/server_bg.wasm`. `npm run publish:bgs` now uploads and verifies the whole build before switching the viewer.

Run `npm run build`, then `npm run publish:bgs -- --dry-run` to review the target and `npm run publish:bgs` to publish. All assets share one immutable bundle directory. The script verifies their public contents before switching the viewer and saves the old metadata for rollback. It also removes the obsolete `undo` lobby option; the engine always locks undo when information is revealed. Existing saves with unrestricted undo seal their previous undo history once on load.

BGS compresses the JavaScript and WASM automatically. Send original bytes; do not create `.gz` files. The publisher sets `scriptBytes` for the progress indicator. Keep the engine bridge compatible with the deployed engine; this packaging change does not require an engine upload.
