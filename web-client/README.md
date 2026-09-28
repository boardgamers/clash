# Clash of Cultures — web viewer

First playable slice of a Svelte/TypeScript interface with a Three.js board. The Rust engine remains the rule authority.

## Run locally

Requirements: Node 24+, Rust with `wasm32-unknown-unknown`, and `wasm-bindgen-cli` matching Cargo.lock (currently 0.2.100).

```sh
cd web-client
npm ci
npm run build:bridge
npm run dev
```

Open http://127.0.0.1:8643. The preview binds to loopback and saves its disposable game under `.engine/preview-state.json`. **New game** resets that game and its local chat. Hotseat follows the active player; the player selector also supports fixed seats and spectators.

No production games, catalog records, or chat rooms are modified. Remaining actions and phases will be implemented in this interface.

## Implemented slice

- Procedural 3D terrain, cities, settlers, tile selection, orbit/zoom, and a top-down view.
- Named resources, storage limits, player scores, action count, city mood, and a short field guide.
- Private objective hand with both alternative conditions and their completion timing, read from the Rust definitions. When the engine offers an eligible objective, the player can claim points or keep the card directly in the action panel.
- Collect resources: Rust supplies legal tiles, resource alternatives, capacity, total gain, waste, and an activation warning. Confirming submits the original Rust action format.
- Research: a searchable tree with pictograms, short effect summaries, prerequisite branches, costs, bonuses, and unlocked buildings. Rust supplies the graph, full rules, payments, and action payloads; selecting an advance opens its full rule text before confirmation.
- End turn and undo, with state and journal reconciliation.
- City management: buildings with visible effects and requirements, Port placement, combined recruitment of standard units, and one-city happiness actions. Payments and legal actions come from Rust; repeat activations warn about mood loss.
- Settler movement on revealed land without enemies, movement completion, and founding cities. Destination rings and map clicks work alongside accessible destination buttons.
- One-resource bonus choices (including Temples) and yes/no decisions. These remain private to the player who must answer.
- Buildings appear around city centers; recruited units have separate positions and basic type markers.
- BGS registration/lifecycle, player/spectator handling, theme, avatars, and the shared chat controller/panel. Chat sends, failed-send drafts, updates/deletions, mentions, read reports, editing and translation handlers are supplied by the protocol package; the host decides which features are enabled.
- Journal entries use action and resource pictograms. Setup is summarized as player, civilization, and original starting tile. Text equivalents are published with `replaceLog` so undo cannot leave stale entries.
- Full-width board with floating action controls and dismissible journal/chat panels. The layout stays within the viewport on desktop and mobile; panels scroll internally. City/action controls are keyboard accessible, and the board renders only when changed.

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

`dist/viewer.js` contains the interface, styles, and Rust WASM bridge in one uploadable file. It exports `window.clash3d.launch(selector)` and uses string state/move payloads, matching the existing engine. Configure `topLevelVariable: "clash3d"`, `fullScreen: true`, `chat: true`, and `replayable: false`. No extra runtime scripts or stylesheets are needed. Fonts currently load from Google Fonts; system fonts remain available if blocked.

## Publish to BGS

After building and verifying, use the admin API token from `BGS_ADMIN_TOKEN` or `~/.bgs`:

```sh
npm run publish:bgs -- --version=1 --dry-run
npm run publish:bgs -- --version=1
```

The script uploads the viewer, verifies the hosted bytes, then updates only the version's viewer configuration. It preserves the engine and visibility settings, checks for concurrent viewer changes, and saves the previous version document under `.engine/releases/` for rollback. Publishing is separate from pushing Git commits. The GitHub workflow builds and tests the viewer and retains the bundle as an artifact; it does not publish automatically.

## Next slices

The controls do not yet cover army/fleet movement, exploration, unit replacement/leader recruitment, wonders, action cards, cultural influence, combat, civilization selection, or most status/event choices. Free/custom-action variants and multi-city happiness selection are also pending. Models for buildings and units are preliminary. Full replay and analysis controls are not advertised. The interface text is currently English; existing game translations need to be wired into the new presentation layer. Audio and advanced accessibility preferences need a separate pass.

Before public release: complete action/phase coverage, verify against an actual BGS staging game (including reconnects, moderation, translation, and read-state persistence), and test larger multiplayer boards on mobile hardware.

## Architecture

`server/src/web_view.rs` and `web_view/actions.rs` expose read-only rule queries; `game_api_wrapper.rs` exports them to WASM. Queries consume the same player-filtered state BGS sends to the viewer. They neither mutate the game nor send moves. The server always validates actual moves. `src/bridge.ts` initializes the embedded WASM; `.bridge/` and `.engine/` are generated by `build:bridge`.

`src/controller.ts` coordinates state, choices, and the BGS commands. `src/App.svelte` owns the HTML interface. `src/board.ts` owns and disposes the Three.js scene. `src/viewer.ts` is the production entry. `src/local-host.ts` and `scripts/preview-api.ts` are development-only.

The protocol dependency is AGPL-3.0-only. The engine retains its existing licensing.
