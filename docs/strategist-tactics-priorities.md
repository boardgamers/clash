# Strategist / tactics listener priority bands

Release 0.1.240 / engine 0.4.171 includes a deterministic fix for Check CI run
45306584337. The random-actions test exposed a genuine registration panic, not a
flaky assertion: `LeaderAbility("Strategist")` and attacking `TacticsCard(24)`
both registered priority 100 in `combat_round_start_reveal_tactics`.
Card 24 is **Negotiations / Martyr** (civil / tactics halves).

`Event` requires distinct origins to have distinct priorities. Persistent response
cursors also use priorities, so dropping this check, overwriting a listener, or
allowing arbitrary ties would be unsafe. Both tactics cards are temporarily
installed on **both** combatants; Cyrus need not be the owner of the colliding
card. Same-origin registrations are a separate existing objective mechanism.

## Ordering contract

- Defender tactics base priorities occupy `[0, 100)`.
- Attacker tactics keep their existing +100 offset, occupying `[100, 200)`.
- Reveal abilities occupy priorities `>= 200`; Strategist uses 200.
- Priority is descending **within each participant**. Participant order, combat
  phases, cancellation timing, attacker-before-defender card ordering and card
  response priorities are unchanged. This is not a new global event scheduler.
- Strategist remains in the reveal phase, before that participant's card
  cancellation/effects, not after reveal or in the later tactics-effect phase.
- Previously saved Strategist responses at 100 match by exact origin as well as
  the old priority and resume at 200. An attacking card must not consume them.

Construction checks every tactics base priority against its band; registration
checks reveal abilities against the pre-tactics band and retains the ordinary
origin/priority uniqueness invariant. Current tactics base priorities are 0–10;
Strategist is the only non-card listener on the reveal-cancellation event. The
catalog regression registers every distinct attacker/defender card pair alongside
**every leader**, on both participants, and deinitializes every registration.

The real-combat regression reproduces the original panic before the fix. It covers
attacking and defending Cyrus, human defenders, both Strategist answers, normal
first-round preparation and a serialized later-round continuation, reload, and old
priority-100 pending decisions. Accepted replacement adds +2; declining preserves
Martyr's successive sacrifice requests for both players. Replacing Martyr itself
must suppress both its cancellation and its sacrifices, so its sacrifice listener
now observes the existing `is_active(..., AllPlayers)` ignored-card check too.
No random tests are suppressed, no fixtures are rewritten, and Aggressor / Great
Architect history boundaries are unchanged.
