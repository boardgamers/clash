# Monumental game lengths — viewer 0.1.203 / engine 0.4.137

The five creation choices are approved for publication with the matching engine.
`web-client/bgs-length-option.json` is the publisher input; the original proposal
is retained as a matching reference. Existing saved games are not migrated.

## Selection and compatibility

A single `options.length` enum selects exactly one mode. Arrays, unknown enum names
and objects combining modes are rejected by serde, including the WASM option parser.
Civilization selection, Builder and balance-patch options remain independent.
There is no ambiguous `Short` alias. Missing length still means serialized `Standard` (displayed as **Normal**, 6 ages);
`Epic` still means 8 ages in ongoing games, archives and replays.

- `Shorter`: four ages and two initial settlers, including civilization-draft setup.
  Nonfinal status phases give one free advance to every player in turn order, then
  a second full pass. The second advance comes from supply, not the event tracker:
  ordinary prerequisites, bonuses and advance listeners run, but no event marker
  is consumed by that advance. Any nested choice retains its ordinary rules.
- `RollToEnd`: seven-age track. Roll before the first player's turn-start abilities
  in each eligible round. The printed thresholds supplied by the user are:

  | Round | End on |
  | --- | --- |
  | VI.2 | 6 |
  | VI.3 | 5–6 |
  | VII.1 | 4–6 |
  | VII.2 | 3–6 |
  | VII.3 | 2–6 |

  The existing RNG/dice path has two faces for each number 1–6, and is therefore
  uniform. It records the roll and uses the normal information-reveal undo barrier.
  A successful roll enters final status immediately, without a player turn.
- `WonderfulEnding`: construction (not acquisition or capture) in those same five
  rounds sets a persistent flag. Every remaining player finishes that round before
  final status. Undoing a reversible construction also undoes the flag.

Every final status phase checks objectives only: no free advances, cards, razing,
government changes or first-player selection. Variable modes without a trigger
finish normally at the end of VII. Existing no-city and withdrawal endings remain.

## State and UI

Optional `variable_end` records the triggered ending and survives saves and replay.
It is absent in all old/default games. `SecondFreeAdvance` is a distinct persistent
status phase, reusing the normal free-advance request/benefit handler. First-pass
preselections remain compatible; the second pass uses ordinary live choices.

The viewer uses the saved length for the age track. The separate Game length
collapsible is removed. Tap/click any age (or use Tab and Enter/Space) to open
anchored rule details; Escape restores focus, and outside click dismisses them.
Hover titles and expanded/current-step semantics expose the same affordance.
The current mode's rules and all five mode names/age counts remain discoverable
there, including Shorter, Normal and legacy Epic. Normal is only a display name:
the serialized enum and creation option's value remain `Standard`.

Roll to End marks VI and VII with dice; Wonderful Ending marks them with a wonder.
VI.1 is explicitly ineligible in the details and its current-age icon is muted.
Details give all five eligible rounds, exact dice thresholds, immediate versus
end-of-round timing, objectives-only final status and the VII fallback.
The round caption shows the applicable dice threshold in Roll to End. Wonderful
Ending shows **Wonder to end** only during VI.2–VII.3; a recorded construction
trigger changes this to **Ends this round**. Both captions open the same details.
Mobile retains the age track on a compact row; details wrap within the viewport
and render above map controls. All 17 catalogs contain the active captions and
rule/log strings (native-language editorial review remains useful). Second-pass
research does not display a spurious event-trigger warning.
The legacy Macroquad client continues to show the current age and round and uses
the same engine rules; its rule-help/localization UI is not expanded here.

## Local verification

- Rust 1.94.1 workspace tests, formatting check and Clippy completed. Clippy still
  reports the repository's existing warning backlog.
- Eleven focused native tests cover the printed track, option shapes, both advance
  passes, incident-marker boundaries, normal bonuses and nested wonder-card choice
  resumption across save/load, final status, construction versus acquisition,
  remaining-player handoffs/drops, and round-start undo barriers (including a
  first-player chooser selecting themselves).
- Frontend type checking, 1,028 tests, catalog parity/source checks and production
  build completed against a locally rebuilt wasm-bindgen 0.2.105 bridge.
- Browser smoke checks (`node web-client/scripts/game-length-smoke.mjs`, after
  building and starting the local preview at http://localhost:8671/) cover
  1,400px and 390px EN/FR layouts, all five modes, actual age/phase-caption taps,
  keyboard activation, Escape/focus restoration, outside dismissal, exact dice
  thresholds, VI.1 inactivity, all five wonder-window rounds, triggered ending
  labels, popover bounds/layering, no horizontal overflow and no page errors.
  Screenshots are saved to `/tmp/clash-age-{roll,wonder}-{en,fr}-{1400,390}.png`.

One intermediate randomized native run failed in the unrelated Tributes /
Acamapichtli action validator; subsequent full runs passed. The failing randomized
state was retained at `/tmp/clash-length-random-action-failure.json`, not added to
tracked fixtures. This is not represented as a warning-free or exhaustive rules audit.

Release verification also checks the approved BGS creation choices against WASM,
including the missing-length Standard default, and runs the mode tests against
the packed 0.4.137 engine. Calendar privacy/history/undo and Voting/Sparta payment
regressions pass against that same package. The release workspace test run passed
(including random actions); the earlier randomized failure above did not recur.
