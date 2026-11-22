# Raiders in the Trade Routes guide

The Trade Routes guide uses optional `raidedSettlers` metadata from the engine's
existing `huns::raided_settler` predicate. Engine units are expanded, so carried
Settlers are included. This public board information is calculated for every
selectable civilization, including off-turn inspection. Saved game data and rules
are unchanged; older views without the field remain supported.

Affected source hexes have one solid orange contour. The trade guide shows only a
Settler icon and “Settlers blocked by Huns’ Raiders”; clicking a hex keeps that same
short warning in the normal tile panel. There is no warning paragraph, unit-ID list,
or eligible-pairings heading/count. Valid coordinate route rows remain.

The orange contour previously floated at a fixed world Y of 0.67 with depth testing
disabled, so it appeared above the water slab (Y 0.01) and over unit silhouettes.
It now uses the same slab-surface calculation as the board, with a 0.04 centerline
offset and a 0.025 tube radius, inset inside the hex perimeter. Depth testing lets
pieces and terrain occlude it. Gold endpoint contours use the same slab-relative
height; route arcs are unchanged. Strategy uses the same ground geometry underneath
its DOM/SVG unit markers, not an elevated warning layer. All valid gold route lines remain visible,
including the Ship's route from a hex shared with affected Settlers. Only the gold
endpoint contour is replaced with orange. Engine pairing and income are unchanged;
protection restores the normal gold endpoint. Selection contours do not stack on
the orange contour.

## Local verification

From `web-client`:

```sh
npm run build:bridge
npm run build
node --test scripts/raiders.test.mjs
node --experimental-strip-types --test src/trade-overlay.test.ts
node scripts/raiders-smoke.mjs
node scripts/raiders-preview.mjs
```

The final command serves the normal built viewer at `http://localhost:8645/`.
It now defaults to a **real spectator**: the host emits no player index and uses
`stripSecret(raw, undefined)`, not Greece's participant-filtered state. Use
`http://localhost:8645/?seat=spectator` explicitly, or `?seat=1` to compare the
original off-turn Greek participant. There is no simulated seat or game mutation.

Open **Show trade routes** (the route icon in the bottom map toolbar), then choose
**Greece** in its **Civilization** selector. The initial inspection follows the
active civilization, Huns; selecting Greece changes only the overlay, never the
seat, turn owner, camera home, or secret filtering. Explicit trade inspection is
retained when reopening the guide. The synthetic board has Greek Ship #8 and passenger
Settlers #6/#7 at D7, a Huns army at D9, and two Happy trade destinations. The
engine assigns only the Ship a route; the tests also verify its route to D9 when
the other destination is Angry. The fixture is derived from a checked-in rules
test, not a live game or private history. The preview records attempted moves in
memory but never executes or saves them.

The smoke test exercises genuine spectator and off-turn participant hosts,
physical 3D/Strategy tile clicks, keyboard/touch, civilization switching, dark/light, and 3D/Strategy at
1400×900, 1047×827 (the reported screenshot size), and 390×740. It checks the map
controls are enabled and inside the viewport, private hand buttons are absent for
spectators, and no moves are submitted. Screenshots are written to
`/tmp/clash-raiders-{spectator,player}-{1400,1047,390}-{3d,strategy}-{dark,light}.png`.

## Spectator controls audit

Regions and its civilization selector, sea routes, Trade Routes (including
Raiders), pirate/barbarian guides and their incident-player selectors, zoom,
Strategy and its key, and fullscreen (when the host supports it) were already
available without a seat. Pirate blockade marks are automatic; tile inspection
already exposes public terrain, coordinates, city buildings, and units. There
are no separate terrain-label, coordinate, city-range, or unit-model toggles in
the existing in-viewer toolbar to unlock. Account-host preferences are not game
actions and are not being replaced with a spectator seat.

The three-shapes button is **Color-blind mode**, not a menu or collapsed table
controls. The map toolbar is a separate row beneath Journal/Chat/Sound. In the IDE's
embedded browser, the map's percentage height could resolve to the full viewport
instead of the remaining flex height below the header. This put the toolbar below
the screen; focusing a guide scrolled player cards under the header. The map now
fills its positioned board container with explicit insets, verified in the actual
IDE browser as well as the Chromium smoke tests. No global
"always expanded" preference was added. Spectators now omit the participant action
button row rather than displaying disabled actions (or an empty Abilities menu);
public civilization inspection remains available by clicking each player card.
Private hands, own-city management, queues, payment, and participant settings
remain unavailable. No authorization or engine legality checks were changed.
