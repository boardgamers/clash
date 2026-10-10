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

The next viewer update formats victory points consistently to one decimal at most, preserving Great Pyramid’s extra 0.1 without floating-point tails in player cards, score breakdowns or wonder details. The engine is unchanged.

Viewer 0.1.21 fixes Teach Us event warnings in the bundled rules bridge, highlights eligible research choices, and places the Peltasts result beside the tactics card in combat journals. Map pieces open inspection until Move is selected; city dock buttons open the same city details as the map. Ongoing movement stays highlighted with a persistent Finish Move control, and barbarian movement arrows have a contrasting outline. This viewer remains compatible with engine 0.4.39; no engine upload is required.

Viewer 0.1.22 makes possible new barbarian city sites visible without relying on color: thick light/dark hex outlines, a translucent fill, and a city-plus marker with a checkmark for the selected site. The same markers appear in the map guide and incident placement prompts, in both 3D and Strategy view. Engine 0.4.39 is unchanged.

Viewer 0.1.23 keeps Move city visible but disabled for Hunnic cities stopped by entering mountains this turn, with an explanation in the city details. The message follows the saved restriction and disappears when it clears. Engine 0.4.39 is unchanged.

Viewer 0.1.24 previews Undo immediately when the exact preceding state was already received for the current player and turn. The server still confirms every Undo; controls remain locked with a Confirming label, failures restore the last confirmed state, and unavailable snapshots use the normal server path. The bounded in-memory cache is cleared on seat/turn changes and pruned on history branches. Engine 0.4.39 is unchanged.

Viewer 0.1.25 replaces Yes/No prompts with the actual outcomes and explains consequences for optional card and civilization abilities, including Guillotine's permanent loss of unused leaders. Recruitment and civilization details retain unavailable leaders with their status: already on the board, previously killed or replaced, or removed by Guillotine. All supported locales include the new wording. Engine 0.4.39 is unchanged; the bundled viewer rules bridge supplies these presentation changes.

Viewer 0.1.26 stacks descriptive choices in full-width rows so outcome labels and consequences are easier to read. Only plain Yes/No pairs retain the compact side-by-side layout. Engine 0.4.39 is unchanged.

Viewer 0.1.27 uses inline resource icons in choice buttons and their consequence text. Amounts use compact icon-and-number labels while the button keeps its complete accessible name. Unnumbered mood and culture references also receive the appropriate token icon. Engine 0.4.39 is unchanged.

Viewer 0.1.28 colors chat authors by player, adjusting shades for contrast in light and dark themes and respecting custom colors and color-blind symbols. Author buttons still open player profiles. System events appear as centered dividers with a timestamp and no chat author label. The shared BGS chat controller and its editing, translation, mentions and scrolling behavior remain in use. Engine 0.4.39 is unchanged.

Viewer 0.1.29 opens finished games directly on the final state and closes automatic catch-up when a game finishes during playback. Manual replay remains available. Publish with engine 0.4.40, which removes the duplicate game-ended/winner chat announcement while retaining the winner in the journal; BGS supplies the end-of-game system message.

Viewer 0.1.30 replaces the cavalry symbol with a recognizable horse head, consistently across combat dice, journals and unit summaries. Engine 0.4.40 is unchanged.

Viewer 0.1.31 and engine 0.4.41 make the Gold deposits event rule explicitly say “Gain 2 gold”, with the usual inline resource icon in journal rules and translations in every locale. Fire now actually deducts 1 wood when the triggering player has no forest cities, recording a structured resource loss instead of a text-only claim. Existing game history is preserved.

Viewer 0.1.32 and engine 0.4.42 offer retreat only after both armies have resolved their casualties and round-end effects. The map therefore shows the surviving defenders before the attacker chooses whether to continue. Saves already waiting at the previous retreat prompt still accept their answer, apply the remaining casualties once, and do not repeat the same round's retreat question. Publish the matching viewer and engine together.

Viewer 0.1.33 and engine 0.4.43 explain automatic exploration rotations in the journal. When only one orientation of a revealed block is legal, the log names the deciding rule (land units cannot enter water, revealed water must connect to the ships' sea or to existing water, or must touch the map edge), translated in every locale. Rotations the player chooses are unchanged and remain undoable. Publish the matching viewer and engine together.

Viewer 0.1.34 responds to player feedback; engine 0.4.43 is unchanged. Moves that don't depend on hidden information — including Undo, Redo and End turn — are predicted with the bundled engine and shown immediately, then reconciled silently with BGS (`move:result` rejections roll back at once); exploration, draws, dice, battles and the status phase still wait, with a "Confirming…" indicator. Card targets such as Heavy Earthquake's buildings and casualty units can be clicked on the map, and cultural influence has its own map mode for choosing the target and source before rolling. A new **Confirm unit moves** preference (on by default) lets players move on a single click; attacks, exploration, paid routes and ambiguous destinations still confirm. City mood faces stay visible during unit choices and appear in Strategy view, and city buildings, wonders and Port docks no longer clip into the city center. Publish with the updated `bgs-preferences.json`.

Viewer 0.1.35 and engine 0.4.44 explain why a Settler cannot found a city. Instead of the generic "Move to an empty land tile", the found-city button names the actual blocker: an enemy Huns army within 2 spaces (Raiders, unless your own army shares the space), Barren or exhausted land, a Settler still aboard a ship, or the city limit. Translated in every locale. Publish the matching viewer and engine together.

Viewer 0.1.36 and engine 0.4.45 fix collecting multiple resources from a single tile by selecting another tile in between. The engine totals all selections by position, regardless of their order. In normal collection, clicking another resource on the same tile replaces its previous choice, even at full city capacity; Production Focus still permits repeated collections within its limits. The controls, bonus hints and storage previews share the same selection logic. Publish the matching viewer and engine together.

Viewer 0.1.37 and engine 0.4.46 skip the payment prompt when New Ideas or Synergies researches a resource-free advance, including Math, Astronomy and Priesthood discounts. Research bonuses still resolve, New Ideas still grants 2 ideas, and Priesthood's once-per-turn use is recorded. The research confirmation says Free for these choices. Already-saved zero-cost payment prompts remain valid and are labelled Free. Publish the matching viewer and engine together.

Viewer 0.1.38 and engine 0.4.47 explicitly confirm attacks that break Great Diplomat agreements, explaining the 2-culture cost and card discard with resource icons. Capturing an undefended city or settlers now ends the agreement and discards its owner's Great Diplomat, just like a battle with defenders. Empty cities also require the diplomatic payment. Attacks involving a third party preserve the agreement. All supported locales include the confirmation. Publish the matching viewer and engine together.

Viewer 0.1.39 removes the redundant Cancel button from the diplomatic attack warning. The existing movement controls cancel the move; the warning and explicit attack confirmation remain. Engine 0.4.47 is unchanged.

Viewer 0.1.40 keeps full-game replay under the BGS toolbar: the extra bottom recap panel and card/objective notifications are hidden while seeking through the game. Battle results remain available, and ongoing-game last-turn recaps keep their controls and notifications. Engine 0.4.47 is unchanged.

Viewer 0.1.41 restores the player cards and their civilization details in full-game replay, while keeping last-turn recap controls and notifications hidden. Scores and civilization details continue to describe the saved game; board-history frames contain the historical map and pieces. Engine 0.4.47 is unchanged.

Viewer 0.1.42 splits Barbarians and Pirates in How to play and explains their battle rewards with resource icons, including the additional barbarian city reward, pirate rewards even when losing, and the Carthage exception. The wording follows the official Monumental Edition rulebook, pages 27 and 32, and is translated in every locale. Publish with engine 0.4.48, which corrects Epidemics to affect players with exactly two units, as confirmed by the printed E2 card linked in `docs/card-text-audit.json`. Sanitation protection and the extra loss for Roads, Navigation, or Trade Routes remain unchanged. Completed historical events are not replayed or repaired by this release.

Viewer 0.1.43 and engine 0.4.49 fix Sparta comparing unit IDs instead of fighting-unit counts. Its tactics restriction now uses the number of fighters, for both attacking and defending Greece, and logs both army sizes. The combat journal explicitly labels tactics blocked by Sparta, Trojan Horse, Battering Rams, or Mighty Army, including older public logs. Steel Weapons records when its ore-or-gold activation cannot be afforded or is declined; free Metallurgy activation remains explained with the combat bonus. Anarchy explains which Man God abilities are lost with their Theocracy advances, including Absolute Power from Conversion. New explanations are translated in every locale. Publish the matching viewer and engine together; existing historical battle outcomes are preserved.

Viewer 0.1.44 and engine 0.4.50 enforce Technology Trade’s 2-space range when choosing a partner, using the same unit-or-city proximity check as card availability and Inspiration. Both copies of the card exclude distant players even when another nearby player makes the card playable. Regression coverage includes unit and city contact at exactly 2 spaces and at 3 spaces. Publish the matching viewer and engine together. Already-selected trade partners in saved games are preserved.

Viewer 0.1.45 uses the printed English Monumental Edition names for Objectives, Actions, Tactics, Events and Wonders, verified against all 38 objective, 42 action, 58 event and 8 wonder card scans recorded in `docs/card-text-audit.json`. The Markets objective is Mercantile; the three-route objective is Trader. Names are corrected in hands, decisions, completed objectives, journal references, combat and replay, including Great Pyramids. Saved engine names, card IDs, commands, objective readiness and historical outcomes remain unchanged; Great Migration is distinguished from the Migration event. Existing locale keys remain available alongside printed-name aliases. Engine 0.4.50 is unchanged.

Viewer 0.1.46 and engine 0.4.51 add an Effects indicator with a count and a rules dialog for active public effects, including Trojan Horse, Great Diplomat agreements, Negotiations and public wonders. Scopes distinguish shared effects, affected players and the current player's temporary action effects. Great Seer assignments remain private. Public-wonder draw choices display the wonder's effect, construction cost, required advance and points; Great Mausoleum discard previews include event rules and both uses of action cards. Trojan Horse now requires another human player's defended city, as stated on printed E42, and is no longer offered against barbarians. Existing saved activation prompts remain valid: decline an already-pending barbarian activation to continue without paying. Publish the matching viewer and engine together; historical battle outcomes are preserved. Cultural influence warns before rolling when another eligible city has a lower range cost, offers a source switch with the culture saving, and requires an explicit second click to retain the more expensive source. Automatic movement-group selection and switching seats during movement now select the whole compatible group instead of only its first unit.

Viewer 0.1.47 and engine 0.4.52 label active effects with their source event or action card. Source names use the same quiet link style as advance references and open the full source rules without changing the game. Event sources include their printed E number; action-card sources use the played copy from public history, ignoring undone actions. Older effects without a recoverable copy show the common action rules without inventing a battle use. Each effect has its own padded block with spacing between blocks. Saved effect data and game outcomes are unchanged.
# Viewer 0.1.48

Component styles are bundled into the viewer script because BGS does not load a separate stylesheet. This restores active-effect spacing and source-label layout, as well as the cultural-influence warning styles, in the published game.
# Viewer 0.1.49 / engine 0.4.53

Fanaticism's free infantry now requires losing a battle fought in a city with a Temple. The engine records the Temple condition before capture, so it still applies when that city changes hands. The destination may be any eligible city you own. The advance text now says “If you lose that battle” to make the shared condition explicit. Source: official Monumental Edition rulebook, page 31.
# Viewer 0.1.50

Replay keeps the action toolbar and Journal/Chat controls visible. Research, city management, collection, movement, happiness and available abilities can be opened for inspection; submitting actions, undoing and ending the turn remain disabled. Replay transport and the journal have separate space above the bottom controls on desktop and mobile.
