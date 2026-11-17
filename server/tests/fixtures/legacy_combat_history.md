# Legacy combat replay regression

Synthetic seeded game (`historical-combat-0`), generated locally with legal AI
choices and legacy combat sequencing; no production game or player data.
`legacy_combat_history.json` contains only replay input: setup metadata and 217
ordinary recorded actions. It includes completed battles and a pending battle at
217. Cursor 167 precedes another battle; cursor 216 precedes the final battle.

The history was independently replayed with the frozen, downloaded production
engine **0.4.153** (tarball SHA-256
`0f65a03b0610cc8b52b2ce13e12f505bddfb2d680014bf0c5846844e816f9177`).
`legacy_combat_history.expected.json` records its resulting
players, events, RNG, board and turn state. Keep these fixtures frozen rather than
regenerating them with normal golden game snapshots. Tests must reconstruct the
legacy position exactly, finish an open legacy battle normally, and use new
preparation when starting a battle from either a full-save or partial-history
branch. Explicit switches and undo/redo have separate regressions.
