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

Engine 0.4.35 adds public action/event card rules for journal references, discarded card previews including affected players, and Draft's actual Shogunate cost. The matching viewer shows building effects and other civilizations' research, offers Shogunate drafting in Abilities, removes empty raze replay steps, and grounds terrain decorations on their tiles. Great Prophet's Temple cost and Shogunate's independent allowances are explicit in the rules text.

Engine 0.4.36 exposes public pirate spawn positions using the same rules as incident placement. The matching viewer adds an incident-player map guide for first and second ships, and strengthens resource icon colors while preserving the existing city mood styling in normal mode. Color-blind mode uses yellow happy faces to distinguish them from red angry faces.

Engine 0.4.37 exposes public barbarian spawn, reinforcement and movement guides and a validated Free Education research quote. The matching viewer adds horned helmet symbols for barbarians in color-blind mode, an upfront Free Education choice, early Pottery bonus hints, and an initial map orientation with the player’s home at the bottom. Myths protection wording states the number of affected cities. Pirates keep their existing flag.

Engine 0.4.38 clarifies single-city Myths protection as “your affected city”, including already pending payments. The matching viewer gives barbarian infantry and mounted riders curved ivory horns on their helmets, preserving the existing city flag and color-blind symbols.

The viewer update following 0.4.38 makes home-at-bottom rotation an account preference, disabled by default in both 3D and Strategy views. Publish the updated preference declarations with the viewer. Change this preference in the BGS platform settings; the viewer immediately resets the orientation when it changes. Manual camera rotation remains available. The engine is unchanged.

The next viewer update makes barbarian unit badges follow the unit-badge preference in color-blind mode. Horned helmets on the models and barbarian city flags remain visible when badges are disabled. The engine is unchanged.

The following viewer update skips empty end-turn, raze-city and other phase transitions in visual replay and “Since your last turn”. Turn boundaries remain available to select the recap range; actual rewards, card draws, combat and board changes remain replay steps. City dock tiles use less padding and smaller decorative icons. The engine is unchanged.
