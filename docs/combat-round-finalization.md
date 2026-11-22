# Combat round finalization — viewer 0.1.235 / engine 0.4.166

Baseline: `60484d3d`, viewer 0.1.234 / engine 0.4.165. Prepared for release;
publication follows source push and green CI. No production game writes or save repair.

## Rule verification

The Huns source entry in `civilization-sources.json` links the secondary guide
<https://boardgamegeek.com/thread/2684906> and primary photograph
<https://boardgamegeek.com/image/9490522>. The **actual original photograph was
opened and visually read** (retrieved via `api.geekdo.com/api/images/9490522`,
then its `images.original.url`; the ordinary BGG HTML returned a challenge).
Attila L22 says:

> Horse Master — 1st combat round: 1 Cavalry unit in his Army may instead use the
> Elephant Clash ability when triggering a Cavalry Clash icon.

The official Monumental rulebook linked in that same source file was downloaded
and pages 22–23 read using PDF text extraction. Page 22 specifies Cavalry +2 CV
and Elephant: “The value on the die does not add to your CV, but you cancel an
opponent’s hit.” Page 23 determines casualties from **final** CV, with hit-cancel
abilities applied in the casualty step, and ends combat when either army is gone.
This supports replacing the activated Cavalry face plus its bonus, not merely
subtracting two CV. No claim is made to having newly read the secondary guide.

## Cause and implementation

Persistent events finish *all* listeners for attacker before beginning defender.
A priority 99 defender Horse Master listener therefore ran after priority 1
attacker casualty removal. Raising its priority cannot solve the problem.

New battles use these persisted phases:

1. Existing round-end tactics for both players.
2. `RoundEndModifiers`: both players finish hit-changing abilities/payments.
3. `RoundEndFinalize`: hit-dependent rewards, final reports, casualty choices and
   removal, and surviving-leader effects.
4. Existing post-casualty retreat phase.

The original registered round-end event is retained. Copies select its two
priority bands, preserving listener origins and response priorities. The explicit
contract is **priority > 10 for modifiers; <= 10 for consumers of finalized hits**.
Future round-end listeners must respect that contract. This avoids duplicating
ability implementations or changing legacy saved response cursors. Both copies
are reconstructed on resume, including after a JSON reload. Ordinary attacker-
then-defender preparation and all movement rules remain unchanged.

### Complete round-end listener audit

| Listener | Priority | Dependency / phase |
| --- | ---: | --- |
| Attila Horse Master | 99 | Changes own CV and opposing hit cancels; modifiers |
| Phoenician Protector | 92 | Optional paid cancellation, once/battle marker and later refund; modifiers |
| Qin Tactician | 92 | Optional paid cancellation, once/battle marker; modifiers |
| Fireworks | 91 | Optional first-round paid cancellation; modifiers |
| Colosseum | 90 | Optional paid CV increase; modifiers |
| Ahuitzotl Expansionist | 35 | Leader-dependent automatic cancellation; modifiers, before leader losses |
| Solar Eclipse | 10 | Uses final result to award VP and terminate battle; finalize |
| Final hit reports | 9 | Both players’ adjusted totals before either casualty prompt; finalize |
| Pirate ship reward | 3 | Uses hits for gold reward; finalize |
| Choose Casualties | 1 | Removal/selection (legacy reporting order retained); finalize |
| Legacy retreat compatibility | 0 | No new prompts; accepts already-saved responses on old path |
| Dido Sacrifice | -50 | Requires surviving leader/city and no final result; finalize after defender casualties |

`combat_round_end_tactics` is separate: Encircled modifies hits and Surprise
awards its card using the existing tactics resolution semantics. Both remain in
the existing tactics phase, ahead of ability modifiers; this change does not
reinterpret conditional tactics timing. Tactics-reveal sacrifice/withdrawal is
also unchanged. All ordinary hit modifications finish before casualty listeners.

Horse Master's existing conversion calculation is retained, including the
Persian bonus cap marginal calculation. The converted CV lives in `CombatHits`;
final reports contain 8 rather than 12. Original rolls remain 12, unchanged in
history. Existing viewer hit-report handling already displays the adjusted total
and emits no fresh dice animation; no presentation code change was necessary.

## Legacy saves and replay

`Combat.finalized_round_end` selects the mode for the entire battle. Missing means
legacy, including a pending Horse Master prompt whose attacker losses have
already happened. **Those losses are not undone, reconstructed, or repaired.**
An open old battle remains old for all subsequent rounds. This intentionally
preserves old execution, including the known defect, rather than silently
changing or invalidating its recorded response sequence. A separate explicit
repair would need user authorization and is not supplied here.

`finalized_combat_from` follows the independent asynchronous-preparation history
boundary pattern. New games use zero. Old live saves record the current history
index at their first new battle. Replay lacking the marker stays legacy through
recorded history. Playable replay cursors and fresh branches clamp the boundary
to the next action; redo preserves it. Undo patches exclude the boundary. This
marker does not change the preparation-mode marker. Historical golden fixtures
explicitly retain legacy mode; synthetic regressions exercise the new mode.

## Regression coverage

Native and real locally built WASM tests construct a public synthetic battle from
an existing small test board, not a private production snapshot:

- India: Flanking, 5 Infantry / 2 Elephant / 5 Infantry, all symbols blocked by
  High Ground: 12 CV.
- Huns: Attila + Cavalry, activated 2 Cavalry / unactivated 6 Infantry, paid Steel
  Weapons: 12 CV.
- Before defender Horse Master: both sides still have two units. Accepting gives
  Huns 8 CV, India one canceled hit, and one loss each. Both surviving armies
  reach round two, with no second Horse Master offer.
- Declining preserves the original totals and ends in a draw.
- Attacker Horse Master followed by defender Qin payment: both armies remain
  untouched until both decisions resolve, including across JSON reloads.
- No Attila / no activated Cavalry, extra dice with exhausted Cavalry activation,
  disabled symbols, and capped/uncapped CV contributions.
- Both final hit reports, original dice/history preservation, adjusted viewer CV,
  and no fake reroll animation.
- An explicitly legacy pending defender prompt retains its already-removed
  attackers across reload; no attempted repair.
- Historical replay/branch and undo/redo tests assert both independent mode markers.

The new defender survivor assertion was also run against the preserved legacy
path: it fails with **0 attackers instead of 2 before the prompt**. The same
assertion passes on the new phase path.

### Local validation

- `cargo test -p server --quiet`: 569 passed, 1 ignored, no failures.
- `cargo check --workspace --quiet`: passed (server and native client).
- `bash web-client/scripts/build-bridge.sh`: rebuilt the local WASM engine.
- `node --experimental-strip-types --test src/*.test.ts` in `web-client`:
  1,188 passed, no failures, against that rebuilt engine.
- `npm run check` in `web-client`: zero errors and warnings.
- `git diff --check`: passed.

Full-suite testing also covered reveal-time battle termination. That path now
enters the matching new modifier phase instead of the legacy `Default` phase;
otherwise the phase lookup would reject it. No Tactical Retreat destinations,
movement rules, or withdrawal behavior were changed.

## Release regression additions

`horse-master-smoke.mjs` runs the production viewer/controller and built WASM
at 1400px and 390px. The normal choice UI accepts Horse Master and selects a
Cavalry casualty: final values 12/8, one hit each, original dice unchanged, no
roll-animation class, one surviving unit on each side and the retreat prompt.
CI runs this smoke after building the viewer.

`legacy_horse_master_165.json` is frozen output from the published 0.4.165
package, generated only by the explicit maintenance script. It preserves the
old pending Horse Master outcome, including already removed attackers. Its
separate legally generated from-init history verifies full and partial replay
outcomes and the next-action boundary. The synthetic battle is **not** claimed
to be a from-init replay history: that exact combined scenario remains outside
this fixture. Native boundary tests cover new battles after replay and undo/redo.

Release keeps achievement catalog revision 8 (46 entries), nonfaction art revision
4 and all 60 live asset URLs/bytes unchanged. The faction manifest lockfile hash
changes only because the viewer package version changes; no art input changes.

Frozen fixture provenance: published engine archive SHA-256
`47601b7c4bff38e08cc03aadcaa47ecd35a860e0272ebdd70743d1e105c97a45`.
The new browser smoke and all npm ancillary suites pass. Pinned Rust 1.94.1
workspace check, formatting and ordinary workspace/all-target Clippy pass.
An additional stricter `-D warnings` Clippy experiment fails on existing
repository lint warnings; the repository CI uses ordinary Clippy with
`RUSTFLAGS=''`. No lint settings were changed for this release.
