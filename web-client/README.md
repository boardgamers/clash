# Clash of Cultures — web viewer

Playtest interface built with Svelte/TypeScript and a Three.js board. The Rust engine remains the rule authority.

## Run locally

Requirements: Node 24+, Rust with `wasm32-unknown-unknown`, and `wasm-bindgen-cli` matching Cargo.lock (currently 0.2.100).

```sh
cd web-client
npm ci
npm run build:bridge
npm run dev
```

Open http://127.0.0.1:8643. The preview binds to loopback and saves its disposable game under `.engine/preview-state.json`. **New game** resets that game and its local chat. Set `CLASH_PREVIEW_SAVE` to use a separate local fixture save. Hotseat follows the active player; the player selector also supports fixed seats and spectators.

The local preview does not modify production games, catalog records, or chat rooms.

## Implemented controls

- Procedural 3D terrain, cities, settlers, tile selection, orbit/zoom, and a top-down view.
- Named resources, storage limits, action count, city mood, and a short field guide. Select a civilization to see all six victory-point categories from the Rust scoring rules, with a link to its BGS player profile.
- Private wonder hand with card effects, base costs, required research, and building/ownership points. New wonder and objective draws show a dismissible animated card and a sound cue; loading, changing seats, duplicate states, and undo do not replay old draws. Reduced-motion settings suppress the reveal animation.
- City docks show activation counts. Collection and city management show activation availability and the exact next mood, with capacity and angry-city warnings before confirmation.
- Private objective hand with live resource/research/city/unit progress counters from the scoring rules, distinct objective pictograms, resource icons, and concise completion timing (End of age / During play, with explanatory tooltips), read from the Rust definitions. When the engine offers an eligible objective, the player can claim points or keep the card directly in the action panel.
- Collect resources: Rust supplies legal tiles, resource alternatives, capacity, total gain, waste, and an activation warning. Confirming submits the original Rust action format.
- Research: a searchable tree with pictograms, short effect summaries, prerequisite branches, costs, bonuses, and unlocked buildings. Rust supplies the graph, full rules, payments, and action payloads; selecting an advance opens its full rule text before confirmation.
- End turn, undo and redo, with state and journal reconciliation. Redo appears while an undone action is still available.
- City management: buildings and Port placement, combined recruitment including leaders, supply replacements, and multi-city happiness with legal free-action variants. Payments and legal actions come from Rust; repeat activations warn about mood loss. Size and mood use accessible pictograms. The original capital is identified from the map setup, never city ordering, with a small gold crown on its model.
- Unit selection highlights eligible hexes and opens a shared picker with enlarged portraits of the actual board models. Location tabs, selected counts and passenger labels work for event casualties and movement; selecting passengers clears their carrier, and deselecting all units keeps the hex open. Decision confirmation stays visible on phones; city/player strips make room during map interaction.
- Group movement for settlers, armies, and fleets: legal destinations, attacks, embarkation by carrier, disembarkation, Navigation, movement completion, and founding cities. Destination and collection options highlight their map tiles on hover or keyboard focus. Exploration uses the engine’s forced placements or offers both legal orientations with miniature maps and a board preview before confirmation.
- Action cards (civil effects and tactics descriptions), wonder construction, cultural influence, and available civilization/research/wonder abilities. Free collection and happiness variants use the original engine action type.
- Every persistent request variant has controls: adjustable payments, resource rewards, advances, players, positions, unit types/groups, structures, hand cards, government changes, yes/no, and exploration. Selection previews check membership, counts, special restrictions, and payment validity without executing a move. Combat casualties/tactics, incidents, and end-of-age decisions use these same controls. Civilization selection is also available. The **Choose 1 of 3** setup option deals three non-overlapping civilizations to each player. All players choose privately at the same time; locks survive reconnects. Civilizations reveal together and the map, starting cards and cities are created only after everyone has locked in.
- Tile decisions can be selected directly on the board; unit decisions open the selected hex’s portraits and structure choices stay expanded. Every choice requires confirmation. Mobile decision panels and reference dialogs use compact spacing while keeping rules expandable.
- Civilization emblems, seals and accents distinguish players against neutral light or dark panels.
- All 15 playable civilizations have architecture, ships, clothing and military equipment variants shared by the board and unit portraits. Roofs, temple profiles, hulls, sails, shields and headwear vary while building functions, civilian/military silhouettes and ownership colors stay consistent. Culturally influenced buildings retain the city’s architecture and show their owner on the foundation. Models are generated locally without extra asset downloads.
- BGS registration/lifecycle, player/spectator handling, theme, avatars, and the shared chat controller/panel. Chat sends, failed-send drafts, updates/deletions, mentions, read reports, editing and translation handlers are supplied by the protocol package; the host decides which features are enabled.
- Journal entries use high-contrast text and action and resource pictograms. Setup is summarized as civilization and original starting tile. Text equivalents are published with `replaceLog` so undo cannot leave stale entries.
- Full-width board with floating action controls and dismissible journal/chat panels. The layout stays within the viewport on desktop and mobile; panels scroll internally. City/action controls are keyboard accessible, and the board renders only when changed.
- Small, persistent face icons show each city’s mood on the map; smile, straight mouth, and frown distinguish all three moods without relying on color. City names and mood text appear on hover or keyboard focus. Map targets highlight on hover; cursors distinguish selection, unavailable targets, and camera dragging. City and unit models are selectable directly. Dragging or pinching cannot accidentally select a tile, and Escape closes floating action, journal, and chat panels.
- The ship map control traces connected seas and Navigation shortcuts along the edge; it is a reading aid, separate from engine-validated movement destinations.
- Quiet, throttled tile/control hover sounds, card-draw cues, and synthesized confirmed-action sounds follow BGS's global `sound` preference. Muting stops active and scheduled notes immediately; sound starts only after interaction, and loading or reconnecting does not play past actions.
- BGS's global `colorBlind` preference adds matching ownership symbols to player cards, city flags, and unit groups, with a distinct player palette. Resources keep their icons and labels; terrain keeps its physical shapes.
- Sound and colorblind toggle buttons sit beside Journal and Chat on desktop and write to BGS's global preferences. The Strategy button switches between 3D and Strategy, saves `mapView` (`3d` or `strategy`) with BGS's `updatePreference` command, and restores it from incoming preferences. Legacy `2d` preferences open Strategy. The Strategy view pans without tilting and keeps your original home side at the bottom, including in replay. Spectators retain the standard orientation. Exhausted tiles have hatching and an explicit badge showing their original terrain in both map views. Player cards stay visible during desktop tile inspection; mobile hides them to leave room for the map. Fullscreen replaces the camera-reset button where the browser supports it; optional unit badges remain in BGS preferences. The standalone preview emulates persistence locally for these same settings.
- Journal entries use civilization emblems and names, signed icon deltas for costs/gains, grouped age/round headings, and a combined research/card-draw entry. Unknown rule details remain visible, and the BGS text journal gets the same faction-based summaries.
- How to Play uses readable, theme-aware text and icons.
- Hover or keyboard-focus journal coordinates to highlight their map tile. Click to center the camera and pin the location without selecting a game action; interacting with the map or pressing Escape clears the marker.

The preview host uses a real iframe and `postMessage` to exchange BGS events. Its chat is local test data, not a connection to a live BGS room. Production BGS integration still needs a staging playtest.

## Build and verify

```sh
npm run check
npm test
npm run build
# From repository root:
cargo test -p server --lib web_view
```

After building, `http://127.0.0.1:8643/?built` loads the production IIFE bundle inside the preview host.

`dist/viewer.js` contains the interface and styles. Upload `dist/server_bg.wasm` beside it for the Rust bridge. It exports `window.clash3d.launch(selector)` and uses string state/move payloads, matching the existing engine. Configure `topLevelVariable: "clash3d"`, `fullScreen: true`, `chat: true`, and `replayable: true`. No extra runtime scripts or stylesheets are needed. Fonts currently load from Google Fonts; system fonts remain available if blocked.

## Publish to BGS

After building and verifying, use the admin API token from `BGS_ADMIN_TOKEN` or `~/.bgs`:

```sh
npm run publish:bgs -- --version=1 --dry-run
npm run publish:bgs -- --version=1
```

The script uploads the viewer, verifies the hosted bytes, then updates the version's viewer configuration and preference declarations from `bgs-preferences.json` and civilization setup choices from `bgs-civilization-option.json`, and game length choices from `bgs-length-option.json`. The shared `sound` and `colorBlind` declarations expose BGS's global controls; `mapView` is saved per user for Clash. Obsolete scale, zoom, and color-profile controls are replaced. The script preserves the engine (unless passed `--engine=/path/to/package.tgz`), visibility, and unrelated preference declarations, checks for concurrent changes, and saves the previous version document under `.engine/releases/` for rollback. Publishing is separate from pushing Git commits. The GitHub workflow builds and tests the viewer and retains the bundle as an artifact; it does not publish automatically.

## Remaining playtest work

Automated scenarios cover a complete six-age sequence, mixed rewards, government changes, combat casualties/tactics, wonder construction, action cards, influence, civilization abilities, army/fleet movement and transport, leaders/replacements, and free/multi-city actions. This is playtest coverage, not an exhaustive check of every civilization/card interaction. Ordinary research, construction and recruitment still choose the engine's first valid resource payment; explicit payment requests allow resource adjustments. Models for buildings and units are preliminary. Replay shows the most recent 192 recorded public board positions. Existing saves start recording on their next action; earlier positions are unavailable. Catch-up opens unseen opponent actions with Back/Next, 2.5-second autoplay (longer for battles), a saved per-player autoplay preference, and Skip. The Last turn shortcut opens the latest opponent turn paused; both modes stay open at completion and replay only their selected range. Captions and map highlights use public board changes, and manual steps animate unless reduced motion is enabled. Battles show recorded dice, hits, cancelled hits and casualties in a compact panel, with a pulse on the battlefield. Battle results stay visible during manual stepping, and autoplay allows 4.4 seconds per battle update before continuing. Analysis branches start from the current position, resample hidden state independently, and preserve supported public pending choices; private continuations, remembered Spy information and refilled card piles remain unsupported. The interface text is currently English; existing game translations need to be wired into the new presentation layer. Advanced accessibility preferences still need a separate pass.

Before public release: play through unusual civilization/card combinations, verify against an actual BGS staging game (including reconnects, moderation, translation, and read-state persistence), and test larger multiplayer boards on mobile hardware.

## Architecture

`server/src/web_view.rs`, `web_view/actions.rs` and `web_view/decisions.rs` expose read-only rule queries; `game_api_wrapper.rs` exports them to WASM. Queries consume the same player-filtered state BGS sends to the viewer. They neither mutate the game nor send moves. The server always validates actual moves. `src/bridge.ts` initializes the embedded WASM; `.bridge/` and `.engine/` are generated by `build:bridge`.

`src/controller.ts` coordinates state, choices, and the BGS commands. `src/App.svelte` owns the HTML interface. `src/board.ts` owns and disposes the Three.js scene. `src/viewer.ts` is the production entry. `src/local-host.ts` and `scripts/preview-api.ts` are development-only.

The protocol dependency is AGPL-3.0-only. The engine retains its existing licensing.

Viewer releases: see [uploading the complete viewer build](docs/viewer-publishing.md).

Game length defaults to six ages. Epic games use ten ages; existing saves without a length option retain six. The viewer age track follows the saved option. Publish an engine that supports the length option alongside the setup metadata.

The opt-in `skipRazeCity` player setting is saved in the game by the engine. End-of-age processing keeps that player's cities automatically, including when another player advances the game while they are offline. Analysis branches clear automation settings. The viewer migrates an existing enabled account preference the first time the player opens each game after upgrading (an explicit per-game opt-out takes priority). The decision checkbox saves the setting and explicitly answers the current choice after acknowledgement; loading saved preferences or settings never submits a move.
