# Explicit withdrawal and automatic decisions

This is an asynchronous-platform product policy, not a claim about printed
Clash rules. **Elapsed clock time is not withdrawal.** Only an explicit platform
`dropPlayer` notification (or a save already containing that platform flag)
authorizes dropped-seat automation. Cities and units remain on the board as that
player's pieces. They do not become barbarians, and no full-turn AI is installed.

## Policy

* Continue with two or more surviving human seats; retain seat indices. Skip
  future normal turns of dropped seats. Complete an already-started transaction
  before stopping movement/ending that seat's current turn.
* End immediately with one survivor; all-withdrawn games are terminal with tied
  rankings. Draft withdrawal retains its existing deterministic offered-choice
  handling and now also ends when only one survivor remains.
* At a persistent decision, use the actual responder, not the normal turn owner.
  Stop at living-player input, including a living defender of a dropped attacker.
  Existing saved living-player settings/queued choices remain separately honored.
* Decline optional payments, optional zero-card/unit/position selections, and
  Boolean invitations. A dropped defender does not buy Steel Weapons or play
  optional tactics. `false` on a retreat invitation means continue the battle.
* Mandatory choices are **not passes**. Use the first valid minimum-cardinality
  selection in the request's stable order, respecting unit/card/structure
  validators. Casualties still die; a defeated city owner still receives and
  places an eligible mandatory settler. This is a deterministic default, not an
  optimizing combat AI or a promise to preserve the strongest units.
* Single choices use the first offered value; mandatory resource rewards use
  the validated engine default. Mandatory government replacement uses the first
  eligible government and required advances. Exploration uses the revealed
  block's base rotation, not a hard-coded global rotation.
* Mandatory payment groups are searched together: an optional payment is empty,
  but flexible mandatory costs must leave enough for later mandatory costs. No
  fabricated empty payment and no repayment of an already completed step.
* Existing rules deciding event participation remain unchanged. In particular,
  ordinary status-phase participant lists exclude dropped seats. A decision
  **already pending** when a seat withdraws is retained and completed for that
  saved actor, even if the newly computed participant list excludes that actor.

## Persistence and boundaries

`board_history::settle` is shared by the authoritative ordinary-move boundary and
withdrawal. Each generated response/stop/end-turn uses the ordinary recorded
execution path, then the top of the event stack is inspected again. No blanket
clearing of combat or nested events. Dropped owners' private plans/choices are
removed; living queued plans use their existing revalidation path. Private
`QueueMoves` / `QueueChoice` updates remain non-resolving early returns: saving a
plan alone does not answer anyone's prompt. A subsequent explicit drop or ordinary
response settles the event and only then resumes a living owner's valid plan.

Automatic dropped-seat actions earn no clock increments. A quit transaction
preserves **all** clock counters, including when it resumes a living queued plan.
Undo/Redo does not run a fresh automatic cascade. Withdrawal seals prior undo.
New ordinary-play withdrawals save `(action cursor, seat)` bookkeeping in
`withdrawals`, so history reconstruction applies the drop at its original
boundary and replays the recorded responses without generating them again.
Old saves without these markers retain their historical replay limitations;
this is not a speculative reconstruction of when an old player left.

The selector does not advance RNG or consult a new private-information policy.
Executing a legal selected response may trigger ordinary rule-mandated randomness.
Search is bounded (100,000 selection/payment candidate steps); the automatic
cascade is bounded (512 actions), with a decision-state progress check. Invalid
requests, exhausted search budgets, or non-progressing continuations fail
explicitly rather than looping or inventing an illegal response. Typical real
WASM fixture cascades are tested below the host's ten-second deadline; these
bounds are not a proof that every corrupt/pathological save fits that deadline.

## Legacy and platform limitation

The existing narrow legacy Banking load repair is retained for compatibility,
but now records the mandatory reward instead of cancelling it, preserves clocks,
and starts the successor once. It still requires the same proof of the old shape.
At an authoritative settle boundary, the current player-turn log can also prove
that an old save advanced its normal seat pointer before a withdrawn owner's
suspended transaction completed; that pointer is restored without replacing the
saved event responder or clearing its stack. An unproven mismatched turn-start
continuation fails explicitly rather than guessing whether a successor already
started. This is not a blanket replay repair.

**A release/viewer reload does not repair platform responder metadata for an
already-blocked game.** BGS authorizes moves against its stored `currentPlayers`
before calling the engine and does not issue another quit notification for an
already-flagged participant. Such games require separately authorized,
lock-protected reconciliation of engine data, responders, and preserved clocks.
No production save repair, rewind, forced move, or clock edit is part of this
code release.

## Regression checks

* Native unit tests: Banking current/legacy shapes, first-seat/round handoff,
  idempotence, survivor finish, invalid-handler nonmutation, payment-group
  backtracking, valid minimum selection, and withdrawal replay boundaries.
* Exported JSON round trips: current/future dropped defender Steel/tactics,
  compulsory casualties before living retreat, dropped attacker with living
  defender, status actor attribution, paid government completion, captured-city
  settler, private plans, legacy pointer evidence, last/zero survivors.
* Actual Node WASM tests: `scripts/withdrawal.test.cjs`; also run existing status
  battle, postcombat queue, Tributes, combat preparation/finalization and native
  replay regressions. The selector's match is exhaustive over all persistent
  request variants; that is not a claim of a dedicated scenario fixture for
  every civilization/incident combination.
