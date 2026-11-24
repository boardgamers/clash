# Aggressor eligibility (O23 / O25)

The independently visually checked primary English O23 and O25 faces in the
[Monumental Edition objective deck scan](https://cdn.steamusercontent.com/ugc/1781731512046605829/43DEFEAC6C24E28AFBE67BD5DB2E44AA9B5BD073/)
(Steam Workshop 2651868219, as catalogued in `card-text-audit.json`) both say:

> Immediately: After eliminating at least two Army units in a battle you started against another player.

The local inspection crops were `/tmp/clash-O23-primary.jpg` and
`/tmp/clash-O25-primary.jpg`; these are not repository assets. O23's other half
is Seafarer, and O25's is Government.

The combat-end listener now requires the objective owner to be the attacker,
a human opponent, and at least two opposing army casualties. Winning is **not**
a prerequisite: an attacker can qualify after a loss or draw. The existing
land-battle guard is retained (normal sea battles lose ships, not army units).
The `requires_player_combat` metadata removes only the Aggressor half in Builder;
it is not a substitute for the runtime human-opponent check.

Synthetic native tests exercise the real combat-end offer/claim flow for both
cards and all three results, negative defender/barbarian/one-army/settler/sea
cases, Builder filtering, save/reload before and after claiming, and actual
movement battles with deterministic combat dice. No private production fixture
is used. `scripts/aggressor.test.cjs` also initializes fresh synthetic games
through the public WASM engine, plays actual movement battles, and checks both
cards' offer/claim/save flow after wins, losses and draws, plus defender,
barbarian and one-casualty negatives.

Checks: `cargo test -p server --lib aggressor` and, after
`bash web-client/scripts/build-bridge.sh`, `node --test scripts/aggressor.test.cjs`.

## Existing saves

Loading an already completed award does not re-evaluate its historical combat
and does not remove points. A pending objective prompt already persisted by an
older engine likewise retains its offered choice through reload and claim.
The tests explicitly cover both forms of preservation. The predicate correction
applies when new combat-end eligibility is evaluated, not as a save repair.

`aggressor_eligibility_from` persists the first action using corrected eligibility.
Fresh games use zero. Old histories without the field replay with the historical
winner / land / two-army-loss predicate (including defenders and barbarians),
then adopt the corrected rule for new actions. Partial replay clamps the boundary
to the retained prefix plus one; undo does not erase it, and branching clamps it
to the new action. New histories retain the corrected predicate during replay:
this is not a blanket replay-mode exception. The comparison uses the action
resolving combat-end, not the action that started the battle. Already pending
objective claims remain valid.

Native regressions cover both predicates at the boundary and synthetic saved
battle action/claim replays, undo/redo, and live-save branching for old defender
and barbarian awards. These synthetic battle tests are not from-initialization
historical fixtures. The separate frozen published-engine Architect history
exercises full and partial replay boundary serialization. Builder retains all
38 objective card IDs in order: O23 keeps Seafarers and O25 keeps Government,
so removing Aggressor halves does not alter the shuffled deck or RNG consumption.
An exact seeded setup comparison additionally checks the entire serialized deal
and RNG. Old Builder caches retain the Aggressor halves for historical barbarian
awards and pending claims; new Builder games filter them. The runtime boundary
still prevents new barbarian awards in upgraded old games.

Client-side undo prediction also retains all four independent rules boundaries,
matching native undo patches; otherwise the first undo in an upgraded save could
cause prediction mismatches and drop queued moves. This is covered by both the
existing real-engine prediction suite and an explicit boundary regression.
No production save or historical award was edited.
