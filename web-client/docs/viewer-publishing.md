# Publishing the viewer

Reference links are declared in `bgs-reference-links.json`. Run `node scripts/update-reference-links.mjs --dry-run` to inspect them and omit `--dry-run` to update BGS metadata without uploading a viewer or engine. The script preserves other links, saves a metadata backup, and verifies the rules, links, engine, and viewer after updating. The English rules link opens WizKids' February 5 PDF directly, as requested, instead of a BGG download page. The BGG game link uses Monumental Edition.

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

# Viewer 0.1.51 / engine 0.4.54

Single-route trade rewards include Alphabet's 2-idea option. The compact reward picker now lists complete engine-validated conversions instead of assuming every choice contains one resource. Existing saves and historical rewards are preserved.

# Viewer 0.1.52

The action bar selects an available free Collect or Increase happiness variant first, including Free Economy and Voting. Drawn action cards appear in the existing card reveal queue with their action and battle rules, including end-of-age draws. The Confirm unit moves preference has been removed from BGS and the viewer; moves always wait for confirmation, including for accounts with a previously disabled preference. Engine 0.4.54 is unchanged.

# Viewer 0.1.53 / engine 0.4.55

Assimilator requires winning as the attacker in a Barbarian city defended by at least two army units. Defending your own city no longer offers this objective, and non-army units do not count toward its threshold. Barbarian reinforcement skips city-order decisions when all remaining placements are forced Infantry and supply is sufficient for every city. Limited supply and unit-type choices still require a decision. Owned custom abilities remain visible with disabled controls and their engine availability reason, both in Abilities and leader details. Passive effects keep their automatic behavior. Golden Age controller coverage and all 45 leaders under each government are checked. Existing completed objectives and game history are preserved.

Recruit remains open for inspection when recruiting is unavailable, with a muted label and eye icon, including after all actions have been spent and during replay.

# Viewer 0.1.54

Abilities uses a muted label and eye icon when no custom action, cultural influence or Shogunate draft can be activated. Owned unavailable abilities remain inspectable, including during replay. Engine 0.4.55 is unchanged.

# Viewer 0.1.55

Clicking a movement destination submits immediately, including attacks, embarking, disembarking and Move here shortcuts. If several routes or carriers share a destination, selecting one submits it directly. The removed Confirm unit moves setting never adds a confirmation step. Deterministic status-phase decisions, including pirate placements, payments and Undo, now predict locally and queue without waiting for the backend. Objective completion, combat and hidden draws still wait when masked information is required. Engine 0.4.55 is unchanged.

# Viewer 0.1.56 / engine 0.4.56

Ordinary actions predict locally from the first recorded move, using a stable public playback identity published by the engine. Construction, recruitment, happiness, research, abilities, movement, collection, payments, turn changes and available Undo can continue while earlier deterministic moves await confirmation. Slow confirmations retain the predicted position and queued moves instead of rolling them back after eight seconds. The last round only waits at the actual end-of-age objective boundary. Dice rolls, hidden draws and Spy hand reveals require server results; random-number use and shrinking hidden decks are checked explicitly before accepting a prediction. Server rejection or a different authoritative result still corrects the position and cancels unsent moves.

# Viewer 0.1.57

Huns movement defaults to movable units when a settler shares the focused city. The Move city control can be toggled off without deselecting accompanying units, so a settler can leave while the city stays in place. Changing the selection recomputes legal destinations without submitting a move. City-only and combined Nomads movement remain available. Engine 0.4.56 is unchanged.

# Viewer 0.1.58

Exploration requires two destination clicks: the first outlines all four tiles in the unrevealed region, and the second submits the move. A translated hint explains the second click without adding a confirmation button. Changing destinations or the selected units requires a fresh preview. Ordinary movement and attacks remain immediate. Engine 0.4.56 is unchanged.

# Viewer 0.1.57

Happiness stays open for inspection with the same muted label and eye icon as other unavailable actions. Availability checks affordable happiness increases across all action variants, including free Voting, so spending the last regular action does not incorrectly mute a usable free action. Replay remains inspection-only. Engine 0.4.56 is unchanged.

# Viewer 0.1.58 / engine 0.4.57

Nomads city relocations collapse their matching Move journal entry, retaining resource rewards, action costs and accompanying unit movement. Separate actions, routes and players remain separate; historical structured and legacy logs use the same presentation fix.

Philosophy awards its Science idea before Free Education offers the optional extra payment, so that newly gained idea can pay for the mood token. The research purchase must still spend at least one idea or gold. Food-only purchases and free advances remain ineligible, and declining the payment keeps the Philosophy idea.

# Viewer 0.1.59

Attack destinations now use the exploration-style preview: the first click shows the defending units, leaders and city buildings; clicking the destination again or the Attack button commits the move. Changing the group, route or game state requires a fresh preview. Diplomatic costs remain visible before committing. Ordinary movement remains immediate. Engine 0.4.57 is unchanged.

# Viewer 0.1.60 / engine 0.4.58

Paid Roads moves no longer add Mountain or Forest restrictions at their destination. A group can move again in a new Move action during the same turn, provided it has not fought a battle; unrelated existing restrictions remain intact. The route preview states that Roads ignores those terrain penalties instead of incorrectly warning of a destination stop.

# Viewer 0.1.61 / engine 0.4.59

Steel Weapons activates and adds combat value only in land battles. Naval battles against players or pirates no longer request its ore payment or receive its free Metallurgy bonus. Existing invalid naval modifiers are ignored on subsequent rounds, and the viewer does not describe a Steel Weapons benefit at sea. Troops disembarking into a land battle still qualify normally.

# Viewer 0.1.62 / engine 0.4.60

Action sounds have a higher master level and longer click/action envelopes; hover cues remain subtle and the existing sound preference still mutes everything.

A Trade Routes icon joins the map guides. It draws the selected civilization's current eligible trade pairings using the engine's reward-route calculation, with a civilization selector, endpoint links and empty/unresearched explanations. It is read-only and mutually exclusive with sea, pirate and barbarian guides. Routes reflect the current board and are recalculated when the player's next turn starts.


# Viewer 0.1.63 / engine 0.4.61

Recruitment logs attribute Draft, Sanitation, Sparta and Mercenaries only when the actual payment needs those substitutions. Ambiguous payments retain a single consistent explanation. Old availability-only messages are hidden in the journal without modifying saves. Split primary action rows are combined within the same action and actor; recruitment discount explanations are included in the Recruit row. Separate events, battles, refunds and other players remain distinct.


# Viewer 0.1.64 / engine unchanged (0.4.61)

Trade Routes reward amounts now swap directly between engine-approved combinations. Raising mood, gold or another reward reduces food first, preserves other choices when possible, and always keeps a complete legal reward. Taxes and other reward allocations retain their existing limits. Desktop/mobile smoke coverage exercises India's food/gold/mood choices and verifies the submitted reward.


# Viewer 0.1.65 / engine unchanged (0.4.61)

Prediction and server-acknowledgement fingerprints now canonicalize nested object fields, preserving array order and all visible values. Rust map serialization order could previously make equivalent states compare unequal, intermittently forcing deterministic actions such as Theaters to wait for the backend. Regression coverage checks semantic differences and completing Theaters before either server acknowledgement. The browser smoke test can also accept a local pre-Theaters state via THEATERS_STATE; no live game moves are submitted.


# Viewer 0.1.66 / engine 0.4.62

Wonders shows a Great Architect suggestion when the player holds that card, explaining its culture discount and advance waiver while retaining Engineering. Its button uses the engine-provided card action and remains disabled with the normal reason when unavailable. Undo's retained-action comparison also uses canonical object ordering, so equivalent host serialization cannot discard a valid prior snapshot. Undo also handles historic turns with no action array, which previously threw and forced a server wait. Browser coverage exercises undoing Theaters without a server acknowledgement, and confirms Advance skips its already-quoted culture payment and opens research.

Separation of Power protects only its owner as defender. Wonders no longer make a city count as foreign-influenced, restoring those cities as cultural influence sources, including India’s Proselytism range from settlers. Influence reserves the Arts fee after including interchangeable mood from Great Arena, avoiding unsigned underflow when culture is empty.


# Viewer 0.1.67 / engine unchanged (0.4.62)

Recap captions replace generic choice/payment labels with recorded outcomes: unit placements and losses by player and location, research, gained buildings, payments, happiness changes, and public card-draw effects. Existing saved frames use only their public historical interval, never current hands. Existing action grouping retains follow-up payments without merging different players or combat/event boundaries.

# Viewer 0.1.68 / engine 0.4.63

BGS logSlice now returns entries with public readable text and player metadata alongside the original action. Only the requested active-history interval is summarized; log length, inclusive slice bounds, Undo truncation and saved replay actions are unchanged. The ongoing-game list can use the latest summary without a viewer being open. Empty StartTurn bookkeeping is marked as a phase so it cannot displace the preceding real move. Draw summaries expose card types only, never private card names or private log prose. Existing stored list summaries update on the next move.

# Viewer 0.1.69 / engine 0.4.64

Surviving defenders who fight during their own turn now receive the battle movement restriction, including event-triggered barbarian attacks and attacker retreats. The restriction expires at turn end and is not applied to out-of-turn defenses, uninvolved naval passengers, or ships with Longships. It is applied before combat-end rewards so newly awarded units remain free. Regression coverage includes repeated defenses, turn expiry, saved battles, passengers and Longships.

# Viewer 0.1.70 / engine unchanged (0.4.64)

Map geometry and permanent labels now depend only on visible map content, ownership, capitals and display preferences. Resource changes, research, action availability, activation counters and battle restrictions no longer recreate the whole board. Interaction highlights and tooltips still update normally. Journal rows and their card/research links are rendered only while the journal is open, preserving its scroll position; stable game/view references also prevent unrelated panel updates from recalculating history or rerunning Happiness/Abilities availability queries. Browser regression coverage exercises Theaters/payment/Undo without server acknowledgements and checks that board labels are retained and the journal still opens.

# Viewer 0.1.71 / engine 0.4.65

Read-only WASM view/preview queries retain one parsed game keyed by the exact serialized snapshot, avoiding repeated deserialization and rule initialization while selecting units, resources or payments. Move execution stays isolated. Regression checks cover seat changes, changed snapshots, returning to old states, fresh-query equivalence and non-mutation by queries or moves. Prediction's canonical comparison traverses JSON once without a stringify replacer, and its verified fingerprint is reused for server reconciliation instead of computed again. Hidden-information checks, array ordering, Undo and server validation are retained. Desktop/mobile Theaters/payment/Undo smoke passes without server acknowledgements.

Viewer 0.1.72 / engine 0.4.66 enforces Free Education once per turn, as marked on the player board. Paying consumes the allowance; declining does not. The allowance resets at end of turn, and research previews and descriptions show the same restriction. Publish the matching viewer and engine together.


# Viewer 0.1.73 / engine 0.4.67

Research shows playable action cards beside eligible advances, with card details and direct play. Tech Synergy (engine name Synergies) appears on groups where two advances can be researched at normal resource costs; eligibility comes from the same engine helpers as the card prompt. Aqueduct adds Rome’s normal-cost, free-action Sanitation option to Research, including after regular actions run out.

Payment and reward choice pickers preserve the edited resource amount and automatically rebalance the others among legal combinations. Payment selection avoids unnecessary extra resources, including Carthage’s discounted gold payment for drafting ships. Verified with engine-backed regression tests and desktop/mobile browser checks for research card details/play, Aqueduct and trade rewards. Publish the matching viewer and engine together.


# Viewer 0.1.74 / engine 0.4.68

Selecting a land group and a friendly fleet now offers boarding against the fleet’s combined available capacity. The viewer automatically submits validated embarkation steps without asking players to assign permanent ships; continuation from the same origin/destination consumes only one group move. Capacity and unit movement restrictions remain engine-validated, including when predictions are unavailable or a server acknowledgement is delayed.

Hanno’s Navigator checks whether the ships have moved, without rejecting Hanno for earlier movement. Any eligible ships in his sea space can carry him; internal carrier IDs no longer prevent using another ship, and passenger redistribution preserves capacity. Once-per-turn and post-Navigator ship restrictions remain. Source: printed Hanno card (https://boardgamegeek.com/image/9490516); naval casualty capacity is confirmed by the official rulebook, p. 23, and remains pooled across surviving ships.

Verified with 718 frontend/engine tests, type checks, production build, and desktop/mobile browser checks against both a fixture and a local copy of Stellar-tornado-4358. No moves were submitted to the live game.


# Viewer 0.1.75 / engine unchanged (0.4.68)

Clicking a friendly fleet now immediately boards the selected land group across its available capacity. Removed the redundant Board all confirmation. Desktop/mobile browser and controller tests cover immediate boarding with and without optimistic prediction, while retaining one group-move cost.

# Viewer 0.1.76 / engine unchanged (0.4.68)

Sea spaces occupied by or adjacent to pirates now have persistent dashed red borders with a pale outline in both 3D and Strategy views. Hover and tile details explain that collection and Trade Routes are blocked. The guide follows live and replayed pirate positions, excludes land and unexplored spaces, and ignores escorted Pirate Allies controlled by Carthage. Thumbnail exports omit the overlay. All 16 locales include the explanation; the rules engine is unchanged. Verified with 721 tests, locale validation, type checks, and light/dark browser previews.

Viewer 0.1.77 lowers the pirate blockade border to the sea-tile surface, with minimal clearance to prevent flickering. Engine 0.4.68 is unchanged.


# Viewer 0.1.78 / engine unchanged (0.4.68)

When Port has multiple legal sea spaces, city construction offers Choose port location on map. The dialog closes, eligible sea spaces are highlighted, and clicking one constructs the port with the selected payment. Cancel returns to city management without submitting; changed game states discard stale placement choices. Single-location ports retain direct construction. Verified controller payment/cancel/state-change checks and desktop/mobile browser map clicks.


# Viewer 0.1.79 / engine unchanged (0.4.68)

Public event triggers open a dismissible explanation with official display names, engine-provided rules and resource icons, including events that resolve without a decision. Joining during a pending event also shows its rules. Acknowledgements do not repeat the popup; undo removes withdrawn notices. Research-choice dialogs include the notice so it remains visible above the modal.

Replay displays event explanations only for the displayed historical action interval, including since-my-turn recap and full replay. Recap autoplay pauses at a new event so the rules can be read, then resumes with Play. Public log origins disambiguate same-name cards; incomplete old records show only shared rules rather than guessing a variant. Verified live/replay desktop and mobile browser tests, controller autoplay/acknowledgement checks and history/undo tests.

# Viewer 0.1.80 / engine 0.4.69

Provinces only offers its happiness payment if the captured city still belongs to the attacker. Captures automatically razed at the seven-city limit no longer offer an impossible choice. Existing saved Provinces prompts for missing cities resolve safely; any culture payment is refunded before continuing combat-end rewards. Capture and razing gold remain unchanged.

Verified both responses against a local copy of Willful-vertex-9913 without submitting live moves. Five regression tests cover retained-city happiness, obsolete prompts and automatic razing; all 732 tests, locale validation, type checks and production build pass.

# Viewer 0.1.81 / engine unchanged (0.4.69)

Opening Chat, including switching from Journal, scrolls its message feed to the newest message after the panel becomes visible. Scrolling back while Chat remains open is preserved; pending scrolls are cancelled on close or teardown. Verified with desktop/mobile browser checks using an overflowing message history, type checks and production build.

# Viewer 0.1.82 / engine 0.4.70

Siegecraft is offered only to the attacker. Legacy defender prompts finish without cancelling fortress abilities, refund any selected payment, and explain the skipped ability in the journal. Verified declining and paying against a local copy of Willful-vertex-9913; no live moves submitted.

Movement exposes all affordable payment combinations instead of selecting the first valid payment. Roads therefore offers food/ore and gold substitutions before committing the move. Attack destinations retain their preview after payment selection, and alternative payments remain available in the panel. Regression checks cover attacker/defender Siegecraft, legacy prompts, and choosing Roads payment before submitting an attack.

All 737 tests, locale validation, type checks and production build pass. Desktop/mobile browser checks confirm four payment choices and no submitted move before the attack preview.

# Viewer 0.1.83 / engine 0.4.71

Caesar's Proconsul grants infantry only when its one-gold payment is made. Previously the callback checked the payment list instead of its resource amount, granting an infantry even on decline. The latest D2 capture in a read-only copy of Willful-vertex-9913 confirms that exact sequence; the existing live unit was not removed. Paid/declined regression tests and all 739 tests, locale validation, type checks and production build pass.

# Viewer 0.1.84 / engine unchanged (0.4.71)

Combat dice whose matching unit ability was prevented by High Ground or Mighty Army show a red border and crossed-circle marker in the journal and battle recap. A visible explanation and tooltip name the blocking effect. Matching respects available unit counts and does not flag unrelated or excess faces; historical rolls lacking a recorded roster are not guessed. Combat values and outcomes are unchanged.

Collect preselects one resource choice per eligible tile when the number of distinct tiles exactly equals city capacity, on opening or switching cities. The engine validates the full selection before it appears, and resource choices remain editable. Opening Collect from one of those tiles does not immediately deselect it. Collection is never submitted automatically. Verified blocked-dice desktop/mobile browser checks, selection/payment regression tests, type checks and production build.

# Viewer 0.1.85 / engine unchanged (0.4.71)

Carthage's two routes to a pirate-occupied sea space are explicitly labelled Sail peacefully and Attack pirates. The peaceful option previously appeared only as Water. Verified on a read-only local copy of ClashoftheTitans: ship 4 and its settler move from A8 to E8 with no combat. Desktop/mobile browser clicks submit the peaceful action, and type checks and production build pass. No live move was submitted.

# Viewer 0.1.86 / engine unchanged (0.4.71)

Movement animations follow successive terrain-compatible tile centers instead of interpolating directly across the board. Ships and carried units stay on connected sea spaces; Navigation-capable players can follow the same perimeter waypoints as the sea-route guide. Land units traverse intermediate land tiles. Animation speed scales with route length, intermediate positions respect terrain surfaces, and unavailable historical routes snap to their destination rather than drawing an invalid shortcut. Reduced-motion behavior is preserved; rules and action costs are unchanged.

Verified sea/land routing, reverse travel and Navigation perimeter regressions (745 tests), type checks, production build, and desktop/mobile ship movement using a local copy of ClashoftheTitans without submitting live moves.

Viewer 0.1.87 / engine 0.4.72 ends a game immediately when dropping leaves only one active human player, clears pending decisions, and ranks withdrawn players below remaining players regardless of points. Full admin replay also repairs existing unfinished saves with only one remaining player; replaying to an earlier move does not prematurely end it. Multiplayer pending-choice/turn-skipping handling remains separate from this last-player victory fix.

Viewer 0.1.88 / engine 0.4.73 restores ship exploration beside any reachable sea space, including the destination sea area after Navigation. Unrevealed tiles remain endpoints, and enemy ships still block traversal. Verified against a local copy of ClashoftheTitans; no live move was submitted.

Viewer 0.1.89 adds Influence to the main action bar. Its map flow chooses a destination and specific building first, preselects the cheapest eligible source, then explicitly presents Arts/Influence Culture and their upfront costs before rolling. A single available method still requires confirmation. Desktop, mobile and Strategy browser checks cover building selection, method changes, and no submission before confirmation. Engine remains 0.4.73.

Viewer 0.1.90 / engine 0.4.74 fixes Earthquake's reported target scope: its mood continuation uses the same target as its destruction step. Ordinary Earthquake is shown as affecting only its triggering player; Heavy Earthquake still affects all players. Journal eligibility explanations no longer list unrelated civilizations for ordinary Earthquake.

Viewer 0.1.91 / engine 0.4.75 labels the optional end-of-age government payment as “Decide whether to change government” in the public waiting status, instead of “Choose a payment”.

Viewer 0.1.92 omits empty optional payments from journal prose, including older saved entries. Declining a government change now reads “Keep current government” without the stray “Pay ,” prefix. Actual payments and engine history are unchanged; engine remains 0.4.75.

Viewer 0.1.93 shows public action-card hand counts with a card icon on civilization panels and in civilization details. Counts use the already-redacted public hand arrays; no card identities are exposed. Replay frames without historical hand counts omit the number instead of showing current information. Engine remains 0.4.75.

Viewer 0.1.94 extends public hand counts to objective cards still in hand and unbuilt wonder cards. Each type has its own icon and tooltip on the player panel and an explicit label in civilization details. These counts exclude completed objectives and built wonders; historical frames without counts do not substitute current hands. Engine remains 0.4.75.

Viewer 0.1.95 preserves played civil action-card identities in journal payments and results, with an explicit action-card label and an exact-card rules link. Public hand sizes are removed from player tiles and grouped under a compact Cards in hand row in civilization details. Engine remains 0.4.75. Validated with 756 tests, Svelte checks, production build, and desktop/mobile browser checks including opening the played card from the journal.

Viewer 0.1.96 / engine 0.4.76 prefixes BGS platform move summaries with the acting civilization, including resource gains and fallback labels such as Ended turn. Other affected civilizations retain their recipient labels; the existing 240-character limit includes the prefix. Validated with five Rust platform-log tests and the WASM protocol integration test.

Viewer 0.1.97 / engine 0.4.77 adds a Regions map guide using the engine's public map setup. Region perimeters are outlined in both map views; the selected civilization's starting region is gold. The guide counts other regions containing that civilization's cities for Ruler of the World. The mobile map toolbar uses an icon-only Strategy toggle to keep the guides visible. Validated with 758 tests, type checks, map partition checks for 2/3/4 players, and desktop/mobile 3D plus Strategy browser checks.

Viewer 0.1.98 keeps the region city count inline and only shows that count and the Ruler of the World combat explanation for the selected Greek civilization. Persia instead shows its count including the starting region and explains King of Persia, including its city-size limit. The region boundary guide remains available to all civilizations. Engine remains 0.4.77.

Viewer 0.1.99 / engine 0.4.78 fixes Revolution's independent sacrifices: the first prevents action loss and the second prevents government change. Step two no longer overwrites step one's result, and the government prompt checks the second sacrifice. Saved games already awaiting step two recover the first sacrifice from active (not undone) history. The journal explicitly names the consequence prevented. Validated all four sacrifice combinations and saved-step compatibility, plus the reported game's local copy; no live moves were submitted.

Viewer 0.1.100 fixes mobile map controls wrapping into a tall block and overlapping activity controls. The map toolbar stays on one compact row, with journal/chat above it. Smaller outer margins and an explicit width cap preserve a gap between player panels and the action bar. Redo no longer adds a toolbar column: Undo's menu arrow, long press, context menu, or keyboard Down opens Redo. Verified layout at 320/360/393/430px in both map views, keyboard access, and long-press without accidental Undo. Engine remains 0.4.78.

Viewer 0.1.101 names publicly played action cards in Since your last turn summaries, retaining resource and other outcomes. It resolves the card from the frame's public Playing ActionCard log entry and the public catalog; drawn/hidden cards are never used to infer a name. Checked replay tests and existing public game-history examples. Engine remains 0.4.78.

Viewer 0.1.102 / engine 0.4.79 labels Spies hand choices by current owner, including the opponent whose hand remains redacted in public state. Mixed hands are grouped by civilization and card type while preserving original selection indices. Objective choices show progress for their current owner and end-of-age condition status. Checked ownership/progress against both owners' private views, lack of choice exposure to the opponent, and desktop/mobile layouts.

Viewer 0.1.103 fixes journal “Action card · Card 0” entries. Redacted card-removal records now use the same actor's public Playing ActionCard ID for the name and exact-card link. Genuinely hidden discards stay generic. Verified the reported game's history for all three seats and regression coverage for redacted plays versus hidden discards. Engine remains 0.4.79.

Viewer 0.1.104 removes the duplicate Cultural Influence entry from Abilities and keeps its dedicated action-bar button. Abilities availability now depends only on abilities, and the influence tutorial targets the dedicated button. Checked Svelte and desktop/mobile influence selection flows. Engine remains 0.4.79.

Viewer 0.1.105 / engine 0.4.80 allows reclaiming influenced buildings from another eligible city with normal range and roll boosts. Self-targeted attempts remain unboostable and now say so in the payment preview. Regression coverage verifies remote reclaim and boosting, self-target failure without spending, and rejection of another influenced source.

Viewer 0.1.106 simplifies cultural influence confirmation: editable source/target fields, payment choices with combined costs, no duplicate action and empty-range rows, and a pinned roll summary/button. The influence panel has room for its choices on mobile while longer content scrolls independently. Desktop/mobile map selection and confirmation checks pass, including visible roll-button bounds. Engine remains 0.4.80.

Viewer 0.1.107 simplifies move/attack previews: coordinates appear once in the route, payment choices are labelled and selected visibly, defenders have a compact heading, and terrain details expand on demand. The final move/attack button includes the selected cost and stays outside the scrolling content. Desktop/mobile Roads attack checks verify payment selection does not submit prematurely and confirmation stays visible. Engine remains 0.4.80.

Viewer 0.1.108 / engine 0.4.81 fixes pirate events when Carthage controls pirate ships. Removal only recycles neutral ships; placement checks remaining supply before each ship. All four allied ships suppress placement, and three allies permit only one new pirate. Tests cover zero to four allied ships with both empty and full remaining neutral supply, including supply released by Hegemony.

Viewer 0.1.109 defaults reclaim attempts to the cheapest eligible other city that permits roll boosts. Self-targeting remains available and is the fallback when no boostable city exists; explicit source selections are preserved. The cheaper-source warning no longer recommends losing roll boosts just to save range cost. Influence regression tests and Svelte/build checks pass. Engine remains 0.4.81.

Viewer 0.1.110 splits cultural influence into map-only hex selection, a building choice limited to that hex when needed, source selection, and payment. Single eligible buildings skip the second choice. Undo/Back navigates setup locally. Payment has one direct Influence/Arts button per method with its cost and maximum roll boost; same-hex sources show a no-boost warning. Regression checks cover map-only selection, backwards navigation without submission, and desktop/mobile confirmation. Engine remains 0.4.81.

Viewer 0.1.111 replaces influence payment action text with a numeric action cost and the shared action icon, retaining accessible labels and tooltips. Action details and maximum boost share a row on desktop, with responsive wrapping when space is limited. Engine remains 0.4.81.

Viewer 0.1.112 adds Hand, Discarded actions and Discarded events tabs to Cards. Discards show engine pile order with the top card first, counts and expandable full rules including battle uses. Verified both piles in the local saved-game preview; Svelte and production build pass. Engine remains 0.4.81.

Viewer 0.1.113 / engine 0.4.82 fixes embarked naval defenders rolling dice and being eligible as direct combat casualties. Sea battles now admit only ships as fighters; excess passengers still resolve separately after ship losses. Verified against the reported Joyful-kraken-3479 battle and all 20 combat integration tests, including ship-only dice and remaining transport capacity.

Viewer 0.1.114 / engine 0.4.83 corrects E17 (A Splendid Year) to grant its stated 3 food instead of 2. Printed English card E17 confirms the 3-food reward. Also fixes Conversion when reclaiming an influenced building: the acting player is not treated as their own defender, and attempt listeners run once. Regression coverage checks the food reward and reclaim roll bonus.

Viewer 0.1.115 / engine 0.4.84 separates the natural influence die roll from advance bonuses in journal messages, for example “rolls a 2 (+1 bonus = 3)”, including success and failure outcomes.

Viewer 0.1.116 / engine 0.4.85 shortens influence roll bonuses to “rolls a 2 (+1)”.

Viewer 0.1.117 makes hover more audible, sustains action sounds longer, and adds battle-roll, unit-loss and ship-sinking cues. Action sounds wait for browser audio to resume and respect mute cancellation. Adds a local-only sound tester. Engine remains 0.4.85.

Viewer 0.1.118 shows floating resource icons for bartering, research rewards and other gains without a map origin. City and collection gains retain their map locations; other rewards identify the civilization without inventing a city. Storage waste is excluded. Engine remains 0.4.85.

Viewer 0.1.119 queues brief public-action notices while watching other players live, naming their civilization and research, buildings, moves or rewards. Spectators see all players. Initial snapshots, own actions, repeated states and undo do not produce notices. Replay uses its existing captions. Engine remains 0.4.85.

Viewer 0.1.120 keeps chat accessible during civilization selection: a non-modal picker includes a Chat shortcut and hides while table activity is open, preserving selection when returning. Verified opening chat and returning in a fresh draft. Engine remains 0.4.85.

Viewer 0.1.121 keeps civilization selection beside chat on screens at least 1100px wide, switching between panels on smaller screens. Verified both panels remain visible without overlap on desktop. Engine remains 0.4.85.

Viewer 0.1.122 / engine 0.4.86 fixes Bartering to allow an action, objective, or wonder card from hand, including when no action cards remain. Public cards in play are excluded. Updates advance wording and verifies all three discard types, resource reward, action cost, and once-per-turn restriction.

Viewer 0.1.123 / engine 0.4.87 fixes Rapid Recruitment crashing when Vercingetorix is recruited alongside infantry. Leader placement prompts use the leader name. Existing pending recruitment choices can continue without replay or saved-data edits. Regression coverage checks keeping and deploying both newly recruited units.

Viewer 0.1.124 / engine 0.4.88 corrects Rapid Recruitment timing: recruiting Vercingetorix does not trigger his own ability for him or accompanying units. A later Recruit action in his city, including in the same turn, remains eligible. Clarifies the ability text and tests both cases. Community timing discussion (not an official designer ruling): https://boardgamegeek.com/thread/2814396.

Viewer 0.1.125 / engine 0.4.89 extends the agreed leader-city timing to Leonidas: his own recruitment grants no culture, but subsequent recruitment in his city does. Regression tests also confirm Imhotep gains no idea from his own recruitment and does gain one on a subsequent recruitment. Recruitment listener audit found no other leader-specific post-recruit reward besides Leonidas and Vercingetorix; cost discounts and Imhotep activation already inspect the pre-recruit position.

Viewer 0.1.126 / engine 0.4.90 fixes Loyalty tactics rendering using the Celtic controller as the combat participant. Eligibility now uses the Barbarian army and its combat role while cards remain from the Celtic hand. Regression uses disjoint unit IDs; existing pending choices need only a refresh.

Viewer 0.1.127 / engine 0.4.91 counts Mercenaries targets by distinct Barbarian army hex, not individual units, so selection limits and payment are per army. The prompt explains the grouping. Revolt waiting text explicitly asks for a city for the free infantry.

Viewer 0.1.128 / engine 0.4.92 prefers human-city trade routes over Barbarian routes for Celts with Currency, after maximizing route count. This preserves food/gold options in actual rewards and prevents false unavoidable-food-waste forecasts.

Viewer 0.1.129 names publicly completed objectives in replay and live action captions, including existing recorded frames, without duplicating their completion effect.

Viewer 0.1.130 / engine 0.4.93 preserves mixed mood/culture allocations for variable token payments such as Uprising, while ordinary costs still remove unnecessary resources.

Viewer 0.1.131 / engine 0.4.94 allows New Colonies to disembark into adjacent unexplored regions using land exploration, including orientation choices, terrain effects and the reveal undo boundary.

Viewer 0.1.132 / engine 0.4.95 treats undefended city captures as battles for Captivi, Embalming, Heroic General and battle objectives. Fast War separately requires opposing Army units; defended-city and casualty objective requirements remain intact.

Viewer 0.1.133 shortens the Barbarian map guide and removes its two trailing caveats to leave more map visible on mobile.

Viewer 0.1.134 lets the Barbarian, Pirate, Trade Routes and Regions guides collapse to a title bar while keeping their map overlays visible.

Viewer 0.1.135 / engine 0.4.96 exposes Rituals happiness payment allocation and allows food-distribution events to target other players at their storage limit; excess food is wasted normally.

Viewer 0.1.136 shows current food / storage limit with the food icon beside each food-distribution recipient, without warning text or disabling full recipients.

Viewer 0.1.137 aligns recipient food counts beside the selection checkbox rather than stretching the count across half the row.

Viewer 0.1.140 adds the Market trade-income bonus and its per-opponent-turn limit to the compact building description in every supported language.

Viewer 0.1.141 adds arrows from sea spaces to every adjacent unexplored hex in the sea-route guide, alongside existing Navigation perimeter routes.

Viewer 0.1.142 / engine 0.4.99 removes claimed Great People from the Event discard and excludes Great People in the Action discard from Event reshuffles. Existing saved games are not migrated.

Viewer 0.1.143 uses printed card names in waiting-status labels, matching event popups (for example Major Earthquake).

Viewer 0.1.144 / engine 0.4.100 adds combat estimates to attack confirmation and selected tactics, Siegecraft and Steel Weapons choices. An isolated worker runs 2,048 battles with the real engine, an independent random stream, public information only, and explicit no-retreat / no-additional-tactics and casualty assumptions. Attack previews compare affordable Siegecraft options and declining Steel Weapons. Unsupported decisions show no estimate. Results are cached, selection changes cancel old workers, and simulations stop before capture rewards.

Viewer 0.1.145 enriches cultural-influence recap captions with success/failure, target civilization and building (or unit), without coordinates. Pending rolls stay pending until the boost response resolves; existing public recordings are supported. No engine or saved-data changes.

Viewer 0.1.147 / engine 0.4.101 restricts Great Gardens' resource choices to the city containing the wonder. Plains reached through Husbandry still qualify, other terrain keeps its normal resources, and both previews and submitted collections use the same city-specific rule. The rule applies to existing saves when loaded; completed historical collections are unchanged.

Full-game replay no longer reserves room below chat for the hidden last-turn recap. On mobile this restores a usable message viewport and composer. Last-turn recaps retain their panel spacing. Verified with collection/wonder regressions, 827 frontend tests, all locales, type checking, production build, and phone-sized embedded chat previews. The broader advances test suite still has an unrelated stale cultural-influence log expectation (`rolls a 2` versus `rolls a 1 (+1)`).

Viewer 0.1.148 / engine 0.4.102 uses Anarchy's remaining cubes when forecasting government research events. Eligible government advances no longer show an event warning; ordinary research still does, and the general warning is hidden when available choices have different outcomes. The journal also records the unchanged event counter. Anarchy resolution now preserves other players' cubes and does not add an event marker for card-granted advances that already bypass the counter. No historical games are rewritten.

Verified with 832 frontend/engine tests, Anarchy and Dogma Rust regressions, locale validation, type checking, production build, and a browser preview of Dogma with one event marker remaining. Rule source: the printed Revolution: Anarchy card in the English event deck scan linked in `docs/card-text-audit.json`.

Viewer 0.1.149 / engine 0.4.103 restores missing Anarchy cubes when loading saves affected by the old cross-player removal bug. Surviving Anarchy victory points give the exact unused cube count; recovery adds only missing cubes per player and preserves existing effects and scores. This repairs both research previews and execution without replaying historical moves. Verified against a private backup of the reported game, including Dogma research and Undo, plus 835 frontend/engine tests, targeted Rust Anarchy tests, all locales, type checking and production build.
