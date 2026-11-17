# Asynchronous combat preparation — release preparation

Prepared for viewer **0.1.221** and engine **0.4.154**. This implements the requested
asynchronous interpretation, **not a claim about an official ruling**. Local
preparation is not publication: matching pushed CI and hosted verification remain
mandatory before calling the release shipped.

For a new battle, the ordinary decision sequence is:

1. Attacker's battle-start abilities (including Steel Weapons and Siegecraft).
2. Existing tactics-eligibility and Loyalty steps.
3. Attacker's tactics card or pass.
4. Defender's battle-start abilities, then defender's tactics card or pass.
5. Reveal, card effects (including actual Martyr sacrifice choices), then rolls.

**Special-case exception:** tactics-eligibility effects and Loyalty still run
before either ordinary tactics selection. In particular, Loyalty may let a third
player supply a Barbarian defender's card first. Moving this after the attacker's
choice would change card eligibility (including Scouts); this implementation preserves
those rules instead of silently redefining them. Subsequent rounds retain their
existing eligibility/Loyalty, attacker tactics, defender tactics, reveal sequence.
Battle-start abilities run once, not each round.

These are normal committed responses, not editable private queues. The opponent
can know whether a card was played, but its identity remains hidden until reveal.
The existing masking of the matching discard-pile occurrence is retained.
Other early choices still use QueueChoice. Obsolete `/tactics` plans are removed
on load/prune and never automatically consumed.

## Saved state and history

`Combat.asynchronous_preparation` selects the new sequence for that battle.
Absence means legacy; an already-open legacy battle finishes in the old order.
`CombatRoundStart.preparation` persists the exact first-round continuation,
including both strengths, across saves and ordinary response handling. Start
ability listeners still receive their original `Combat` payload; their serializer
wraps the resulting combat back into that continuation.

`asynchronous_combat_from` is a **format/history boundary**, not a release version.
New games start at zero. A legacy live save records the current action index when
its first new battle starts. Historical replay without a boundary uses the old
sequence through the recorded history and enables the new sequence only beyond
that history. With a boundary, replay uses the recorded switch point while
reconstructing. Before returning a playable replay cursor, the boundary is clamped
to its next action: discarded future history must not force new branch battles
into legacy order. Redo preserves the recorded boundary; a fresh action replacing
an undone future clamps it likewise. The boundary is excluded from gameplay undo
patches: undoing a newly branched battle must not resurrect the discarded future's
switch and make redo run it as legacy. Already-open battles retain their own mode.
Historical
golden fixtures explicitly select the legacy path; focused preparation tests
exercise the new one. No old recorded responses are reordered.

## Local preview

The existing “Tactics · Steel Weapons” scenario starts at Rome's Steel Weapons
payment. Confirming immediately opens Rome's normal Tactics panel; only after
playing/passing does Greece become active. Restart scenario returns to Rome's
payment. The map/layout code is unchanged.

The frozen synthetic 217-action fixture in `server/tests/fixtures/legacy_combat_history.json`
was independently replayed by production engine 0.4.153. Its expected state is
checked through actual `engine.replay`, not by merely applying responses to a
mutated battle fixture. Native tests also cover full-save and partial-history
branches, branch replay, and undo/redo switch handling.
