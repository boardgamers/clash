# Clockwise home placement and save compatibility

New games write the top-level setup discriminator `"home_layout":"ClockwiseV1"`.
It is independent of JSON schema version, player identity, starting player, and
turn/draft order. The same field is carried by full saves and seed/history replay
input. Missing fields deserialize as `Legacy`; legacy serialization continues to
omit the field. Unknown enum values are rejected rather than guessed.

## Physical assignment

The original home definitions remain intact. ClockwiseV1 permutes whole home
blocks (anchor, rotation **and** starting-region side), not just city coordinates:

| Players | Legacy anchor order | ClockwiseV1 anchor order |
| --- | --- | --- |
| 2 | D1, D7 | D1, D7 |
| 3 | E3, A8, G8 | E3, G8, A8 |
| 4 | C1, I1, C8, I8 | C1, I1, I8, C8 |

Anchors are block positions; initial city positions can differ (e.g. D1's city
is D2). Free regions and RNG consumption are unchanged. Civilization overrides
remain attached to the same logical player and use their assigned home's
rotation. Draft completion uses the saved layout, including after reconnect.
All setup lookups (capital, placement, region IDs/owners, Alexander's starting
region exclusion, explorer and analysis regions) take the persisted layout.

The web viewer maps axial positions to `(1.5*q, sqrt(3)*(r+q/2))` with positive
second coordinate pointing down on screen in the default top-down view. Native
hex2d uses the opposite y sign, which `hex_ui::to_screen` flips back. Camera
home-down transforms are rotations, not reflections. Thus increasing player
indices in the new 3/4-player layouts make one clockwise circuit in both views.
The camera continues deriving original homes from setup history; no camera
migration or fixed seat-coordinate table is needed.

## Boundaries

- New initialization selects ClockwiseV1 explicitly in `GameSetupBuilder`.
- Loading an old full state does not rearrange any terrain, city, unit, history,
  or player. Its derived home regions and orientations retain Legacy behavior.
- Replaying an old seed/history explicitly selects Legacy **before** setup;
  replay never silently adopts the new builder default.
- Reload, clone, full replay, partial replay, and continued branches preserve the
  selected layout; replay truncation does not switch it.
- No layout is inferred from cities, map shape, civilization, schema version, or
  seed. Custom maps remain unchanged on load, including empty test maps. This
  field does not make arbitrary custom scenarios reconstructible from seed-only
  replay (which already assumes standard setup).
- Existing games are not migrated. Consumers exporting reduced replay envelopes
  must retain `home_layout` for new games, just as they retain setup options and
  seed. Deliberately deleting it requests the historical format.

## Rules source

The [official Monumental Edition rulebook](https://wizkids.com/posters/repository/wizkids/Clash-of-Cultures---Rulebook---WEB---2021-02-05-compressed.pdf)
page 4 specifies starting-region orientation/sides, civilization-specific region
replacement, and random seating for 3–4 players. **Clockwise turn order is on
page 8**, not page 4. This change aligns physical home order with the existing
logical clockwise turn cycle; it does not implement or change random account-to-
seat assignment.

## Regression coverage

`server/src/map.rs` tests complete block permutations and clockwise angles against
both hex2d/native and web projection formulas. `server/tests/home_layout_tests.rs`
covers all player counts, both layouts, full saves, full/partial replays and
branches, the frozen production Ballcourts history, custom empty maps, random /
choose / draft civilization setup, RNG/deal invariants, and home-region metadata.
`web-client/src/home-layout.test.ts` checks freshly initialized WASM games through
the viewer's actual `positionXY` and every seat's `strategyFrame` transform.
The existing `strategy-camera.test.ts` covers original-home recovery and fitting.

## Release and construction smoke regression

Release targets: viewer **0.1.233**, engine **0.4.164**. Tutorial catalog and
achievement artwork are unchanged. Advance and Construct are the printed action
names; old Research/Build replay titles remain accepted and are normalized only
for display, without editing recorded history.

The construction smoke now explicitly selects its fixture's city through the city
picker. The toolbar intentionally chooses a fresh suitable city (covered by the
controller regression), not necessarily `cities[0]`. Previously City Development
therefore selected an unactivated Happy city while expecting the repeat-activation
warning and an Angry result for the fixture's Neutral, already-activated city.
The production warning and activation rules were correct. Explicit city selection
exercises the intended scenario rather than changing those rules or suppressing
assertions. All three cases (Great Engineer, City Development, ordinary Construct)
retain their resource, activation warning, submitted move, final mood and overflow
checks at 1400, 390 and 320 pixels. The Web viewer CI workflow now runs the
complete construction and research-card smokes alongside the journal smokes.
