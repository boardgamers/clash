# Private planning during postcombat objective choices

The existing action queue can continue while another player chooses whether to
complete an objective after combat. The queue row uses “Choosing an objective”;
there is no separate panel. An existing plan keeps its key, actions and explicit
Wait preference across the settler-placement → objective-choice handoff.

Eligibility requires a single event during a normal round, an optional zero-or-one
objective-card request from the objective-selection ability, and serialized
`combat_turn_owner` provenance from a completed `CombatEnd`. Own pending choices,
unfinished combat, status-phase, mandatory and nested prompts remain excluded.
The optional provenance field defaults to absent and is omitted when absent.
Existing saved prompts without provenance remain unsupported (fail closed); no
legacy save repair, migration, historical backfill or live-game write is included.

Queue keys redact private objective identities and choices before hashing, so the
owner's stripped board agrees with the authoritative board without leaking an
opponent's cards. Plans remain private and queued actions are revalidated only
after every pending choice finishes. Projection does not claim an objective on
anyone's behalf; the owner must explicitly choose their own objective.

The native resistance regression covers combat win/draw/loss and objective
claim/skip, reload, privacy, invalid keys, exclusions and the eventual replay.
Twelve real-WASM regressions run through `web-client`'s `npm test` and Web viewer
CI, alongside the existing status-phase tests:

```sh
cd web-client
npm run build:bridge
npm test
npm run build
node scripts/postcombat-queue-smoke.mjs
```

The focused browser smoke uses the actual production controller, viewer and WASM
with a local platform transport. It checks the settler → objective transition,
private Wait submission and replay after the defender's explicit claim.

## Tributes release blocker

The AI selected its preferred collection modifier once for the entire player,
then applied it to every activatable city. With Acamapichtli in a small city,
that offered Tributes in other cities, which the authoritative executor correctly
rejected. The AI now uses the existing per-city availability filter before
choosing a variant. No leader eligibility, size limit, fee or action cost changes.

`server/tests/tributes_ai_tests.rs` constructs a small public fixture, exercises
both eligible and oversized leader cities, verifies the ordinary fallback in the
other city, and executes every offered collection (including its payment).
The regression fails against the previous generator. `scripts/tributes.test.cjs`
keeps both normal and queued WASM legality strict, rejecting wrong-city and
oversized-city Tributes while accepting the eligible city. It runs in `npm test`.

Release targets: viewer **0.1.231**, engine **0.4.163**. Use pinned Rust 1.94.1,
full workspace tests including random actions, all frontend/localization checks,
and main-commit Check and Web viewer CI before publishing. Preserve catalog
revision 8 (46 definitions), artwork revision 4, all 60 named assets, gallery,
Standard settings, preferences and unrelated metadata. No live-game actions,
rewinds, migrations, repairs or backfills.

The queue browser smoke supports `CLASH_MOBILE=1` and an optional
`CLASH_VIEWER_URL` for testing hosted viewer/WASM bytes with a synthetic local
transport; `CLASH_WASM` selects the downloaded engine. These never submit moves
to production games.
The achievement artwork input manifest's lockfile hash is refreshed only for the
viewer version bump; dependencies and every artwork source/output are unchanged.
