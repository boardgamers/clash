# Calendar: private known-top state

Calendar knowledge is live private state, not an action or a history entry.
`calendar_knowledge` stores the owner, actual turn identity (number of logged
turns), and last disclosed incident. It is outside undo patches. The existing
`undo_limit` remains the knowledge floor and cannot decrease during Undo.

At the first legal Calendar/free-action boundary in the owner's actual turn,
`board_history::execute` records the current top privately and seals preceding
actions without awarding a clock credit. The boundary is after pending events,
automatic responses, and queued actions resolve. Acquiring Calendar mid-turn has
the same boundary: Astronomy is no longer refundable once its complete chain
has ended and the next incident has been privately supplied. The incident drawn
*inside* Astronomy or its Great Mausoleum continuation is not disclosed early.

The turn marker prevents automatic disclosure of replacements after burial or
incident resolution. The owner must explicitly inspect the replacement, which
persists bounded private state and seals undo, but adds no action, log entry,
board snapshot, move count, or clock credit. Empty decks retain the existing
explicit authoritative inspection/refill behavior; no client simulates a refill.

`strip_secret` only removes knowledge: it never creates it. Opponents and
spectators receive no cache. The owner receives a card identity only while it is
the actual known top. Internally the previous identity remains available so Undo
of burial or an incident restores the known card without another disclosure.
An incident that itself reveals a previously unknown top also remembers it.
Filtered states carry `secrets_stripped` so local execution cannot create
knowledge from their shuffled incident pile. The Calendar request renderer uses
the explicit identity, not that pile. Full reconstruction preserves the saved
cache and knowledge floor; historical cursors discard the private cache.

The web controller opens/closes a known-card prompt locally, outside its move
outbox. Repeated opens, Keep, and Cancel send **zero backend calls**, leave all
committed history unchanged, and do not introduce an undo lock. Unknown-card
inspection and old saved transient prompts retain their existing host flow.

A local prompt's paid choice sends `CalendarBury { turn, incident, payment }`.
The host validates ownership, availability, turn, current known top, and payment;
it then expands the command into the existing recorded Calendar start and paid
response. The command itself is never logged. Payments are never predicted.
Known recorded starts do not introduce a new information boundary; legacy
unknown starts still do. Learning the replacement seals the paid burial.

Regression coverage includes turn entry, G5→F5→Stop→peek→Undo/Redo, acquisition,
Great Mausoleum deferral, empty piles, stale burial commands, filtered-state
privacy, full/historical reconstruction, and repeated local peek cycles. The
browser smoke checks local opens and closes in English/French at desktop/mobile
sizes, alongside the existing preview, privacy, and incident Undo/Redo checks.
