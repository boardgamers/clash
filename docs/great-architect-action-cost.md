# Great Architect: construction costs an action

Prepared for viewer 0.1.240 / engine 0.4.171, following 0.1.239 / 0.4.170.
No production save edits or retrospective action deductions.

## Primary rule source

The original **E55 Great Architect** image was opened and visually read:
<https://cdn.steamusercontent.com/ugc/1781731512046920408/E10408D4482864585C2C9FF7B972D4E0FDA0A223>.
The local inspected image was `/tmp/clash-E55-primary.jpg`.

The incident lets the recipient keep the card for 1 culture, otherwise passing
it left for another player to keep for 2 culture. “Immediately when bought”
applies to placing the card in front of its owner with three culture from the
supply, **not** immediately constructing a wonder. It permits disregarding the
prerequisite advance when constructing a wonder, spending the culture on the
card towards that wonder, and then discarding the card. It grants no free
construction action.

## Implementation scope

Public card 155 remains a free modifier. Its wonder cost retains the existing
three-culture reduction and prerequisite waiver (Engineering remains required).
It no longer unconditionally sets `ignore_action_cost` for new construction.
Actual construction therefore pays exactly one action through the existing
wonder payment handler. Preview and execution share that eligibility check:
zero actions cannot consume/strand the card, nor can missing Engineering or an
ineligible city. Selection prompts spend nothing before the construction step.
There is no optional decline added to this mandatory selection flow.

Existing city-specific leader exemptions are unchanged: Nebuchadnezzar and Qin
can still construct a wonder for free in their own city, but not elsewhere.
Japan's Shogunate does not turn this public modifier into an action-cost card or
waive its construction action. Great Engineer 126 is unchanged: its card costs
an action, its accepted construction grants a temporary action offset, and that
building does not activate its city. Other activation timing (including Great
Lighthouse), token storage semantics, and unrelated card effects are untouched.

## Persisted history boundary

`great_architect_action_cost_from` is a 1-based linear-log action boundary,
with `Some(0)` for newly initialized games. A live old save without a boundary
records one when it next starts Architect. Reconstruction of an absent-field
history sets the boundary **after its entire recorded prefix**, so old free
wonders and all subsequent recorded actions remain legal. Returned historical
positions clamp the boundary to the first future action; replacing an undone
future does likewise. Undo patches cannot remove/restore the boundary.

A pending old wonder payment already stores its old `CostInfo` and remains
free. A pending old *wonder-card selection* is evaluated against its originating
Architect play's log index, not its new response index, so it also stays legacy.
Newly played Architect after that pending operation uses the correction. Merely
loading a save does not retroactively charge anything. This deliberately
preserves already-started operations rather than changing their contract mid-prompt.

An independent `aggressor_eligibility_from` is carried by the same shared history
plumbing; the predicates are independent. Legacy golden test games pin missing
Aggressor boundaries to the existing `usize::MAX` compatibility convention.

## Frozen old-engine evidence and checks

`server/tests/fixtures/legacy_architect_170.json` was generated through the
published **0.4.170** engine, using only legal `init`/`tryMove` operations, no
synthetic state edits or private production data. Seed: `architect-history-35`;
ChooseCivilization setup, E55 as the first incident. Research, gathering, status
phase choices, the incident purchase, and a second city are all recorded.
Its cursors are:

- 73: before Architect, three actions available;
- 74: pending old wonder payment;
- 75: Great Wall constructed with three actions still available;
- 78: after Math, Collect, and Irrigation, zero actions remaining.

Native and WASM tests replay this actual history, including all three following
actions. A branch before the card pays one action, and replaying that newly saved
branch also pays once. A branch at pending old payment stays free. Unit tests
also cover saved multi-wonder selection, ordinary 3→2 construction, prerequisites,
zero-action eligibility, Japan, and both free-wonder leaders. WASM checks cover
these public API paths and unchanged Great Engineer action/activation behavior.

Commands used:

```sh
cargo test -p server --lib architect_tests
cargo test -p server --test architect_replay_tests --test great_person_tests
bash web-client/scripts/build-bridge.sh
node --test scripts/great-architect.test.cjs
```

The normal-viewer `web-client/scripts/great-architect-smoke.mjs` clicks the
Architect control and confirms actual payment at desktop/mobile widths, comparing
new one-action construction with a resumed old free payment. Web viewer CI runs
this alongside the unchanged Great Engineer construction smoke.

Additional native and WASM regressions compare old/new undo-redo with replacing
the same future: old redo stays free, new redo and replacement cost one action.
Redo already skips live availability and reconstructs the card event before
evaluating construction cost, so no special redo-mode exception is needed.
