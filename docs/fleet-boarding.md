# Local multi-carrier boarding

A land group can board multiple ships in the **same adjacent sea space** through
one confirmed movement action. Selecting the destination opens the normal movement
panel's passenger assignments; it does not submit a move. Carrier labels identify
the ship and account for existing cargo. Confirm with **Board ship**.

`Movement.Move.boarding` is an optional array of assignments. The outer `units`
list must match the assignment union exactly, without duplicates. Each assignment
has its own owned `embark_carrier_id` or neutral `embark_pirate`; the namespaces are
separate. Nested assignments, payments, attacks, different destinations, already
transported passengers and transfers within a sea space are rejected. Execution
uses a disposable game clone, so a rejected later assignment cannot leave an
earlier load or pirate claim behind. Valid assignments use the existing embark
movement group and therefore share one movement allocation.

Carthage's Pirate Allies claims all neutral ships at the destination, mapping each
neutral ID to a newly owned ID. These claims must respect the four-ship space limit
and eight-ship total limit. The normal movement validation still applies, including
Tactics, moved-unit restrictions, Navigator and civilization-specific capacity
(Viking Longships carry three passengers).

Old movement payloads omit `boarding` when empty, preserving their serialization.
There is no save migration or production-history repair.

## Regression checks

- `npm test` includes durable WASM assignment, invalid-later-assignment,
  deterministic retry, log/action count, undo/redo/branch, queue projection,
  Tactics, Navigator, Longships, namespace and pirate limit checks.
- `npm run test:boarding` rebuilds the bridge and viewer and exercises physical
  clicks on a normal-size map at desktop and mobile widths, for owned and neutral
  fleets with two and four passengers. It uses the supported Strategy map account
  preference, not forced or synthetic click events.
- Web viewer CI also runs the existing embark and ship-landing browser regressions.

The separate old-history replay and hidden-state prediction suites remain required
release checks; the edited local boarding fixtures do not purport to be generated
production histories.
