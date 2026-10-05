# Publishing the viewer

Upload both `web-client/dist/viewer.js` and `web-client/dist/server_bg.wasm`. `npm run publish:bgs` now uploads and verifies the whole build before switching the viewer.

Run `npm run build`, then `npm run publish:bgs -- --dry-run` to review the target and `npm run publish:bgs` to publish. All assets share one immutable bundle directory. The script verifies their public contents before switching the viewer and saves the old metadata for rollback. It also removes the obsolete `undo` lobby option; the engine always locks undo when information is revealed. Existing saves with unrestricted undo seal their previous undo history once on load.

BGS compresses the JavaScript and WASM automatically. Send original bytes; do not create `.gz` files. The publisher sets `scriptBytes` for the progress indicator. Keep the engine bridge compatible with the deployed engine; this packaging change does not require an engine upload.

Engine 0.4.27 adds per-player `skipRazeCity` settings and automatic end-of-age responses. Publish that engine (or newer) with `bgs-settings.json` and the matching viewer. The publisher moves the old declaration out of account preferences, preserves unrelated settings, and verifies the saved settings metadata along with the viewer and engine. Existing opted-in account preferences migrate once when each game is opened.

Engine 0.4.28 allows revising a private civilization draft choice while other players are still choosing. Publish the matching viewer and engine together; the engine exports `canMoveOutOfTurn` and `isLiveUpdate` so replacements do not grant another time increment.

Engine 0.4.29 fixes Star Catalogues offering Great Mausoleum's event discard choice to players who do not own that wonder. Star Catalogues still grants its reward before drawing an event, and Mausoleum owners retain their discard choice.

Engine 0.4.30 makes admin replay throw for saves without supported `action_log` history, empty histories, invalid targets, and illegal recorded moves. Current `log` saves cannot be replayed yet; rejecting them prevents a fresh setup state from overwriting the game. Successful legacy replay remains supported.

Engine 0.4.31 replaces the legacy replay reader with current `log` history. Targets use the history length shown by BGS, the active undo cursor excludes undone moves, and draft civilization choices are reconstructed from setup turns. Recorded actions use normal rule validation; an invalid action or incomplete target throws without returning a partial game. Old `action_log` saves are unsupported.

Engine 0.4.32 allows Great Explorer to grant a free Seafaring advance when no adjacent unexplored region remains. Its optional exploration step no longer prevents playing the card.

Engine 0.4.33 replaces Great Lighthouse's standalone free activation with an optional ship reward after its city activates. Placing the ship costs no additional action or resources and does not activate the city again.

Engine 0.4.34 supplies cultural influence context and upfront costs throughout the attempt. Publish the matching viewer: target and source selection show the full cost, one confirmation carries through the accepted action fee and range payment, and optional rerolls and culture boosts remain explicit choices with the target and roll visible.
