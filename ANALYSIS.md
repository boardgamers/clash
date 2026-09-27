# Ongoing analysis scenarios

The WASM engine exports `createAnalysisScenario(data, { player?, seed })`.
`data` and the result are JSON strings. The platform must authenticate the
requesting seat (`undefined` means a public spectator), supply an independent
cryptographically random seed, persist only the returned scenario and disable
source replay before this snapshot. Subsequent moves and snapshot variations
must use that sanitized state. The export throws for unsupported positions.

`canLaunchAnalysisMode(data)` allows finished games through the existing path,
and permits ongoing ordinary play, civilization selection and movement when
there is no pending event. Pending event stacks, Great Seer assignments and
remembered Spy information are currently excluded. Positions after an Action
Card or Events pile reshuffle are also excluded: the engine keeps lifetime
discard archives, which no longer describe the current deck cycle. These hold private choices
or knowledge that require specific reconstruction rules; copying or dropping
them would be unsafe or change the position's legal continuation.

The scenario preserves public state, the requesting player's hand and known
opponent cards tracked from observable transfers. Hidden departures invalidate
uncertain card knowledge without consulting the departing card identity. Opponent
hands and future piles are sampled from the public card catalogues after
subtracting public discards, completed objectives, built/public wonders and the
requester's known cards and other observable hand cards. Actual hidden pile composition is never consulted.
Unexplored blocks are sampled independently from the published tile catalogue
after subtracting blocks inferred from public terrain, including opposite
orientations and exhausted tiles. This respects canonical tile multiplicities.
The public map and block positions remain unchanged; custom maps that cannot
be reconciled with the catalogue are rejected.

Public facts needed for objective rules survive in a reduced current-round log:
completed objectives, played wonders and combat statistics without private card
fields. Minimal typed observed-hand facts also survive, preserving known
opponent cards when the platform rerolls the sanitized original snapshot. Original actions, free text, private origin/modifier payloads, undo and
redo are removed. The scenario cannot undo before the branch. Player listeners
are rebuilt for simulated hands. Source seed, random state, queued dice results,
messages, custom UI and dropped-seat automation are replaced or cleared.

Validation: `cargo test -p server --lib analysis::tests` exercises seat/spectator
noninterference under changed source seeds, hidden hand/pile composition,
unexplored terrain and private action/undo logs; input immutability; every-seat
continuation, unique legal card draws, branch undo/redo and fail-closed choices.
