# Clash of Cultures achievements

Previously delivered **viewer 0.1.223 / engine 0.4.156** published catalog revision **4**
with **42 definitions**. The previous live catalog was revision 3 (41 definitions),
verified before this release. Artwork remains the existing 41 named achievement
images; Five Objectives shares the Three Objectives image without changing bytes.
Historical delivery receipts remain in `web-client/docs/achievements-*-receipt.md`.

The native Rust evaluator is the single source of truth. The WASM package exports
`achievementDefinitions()` and `achievements(serializedData)` directly, alongside
its existing engine hooks. The web-client tests check the generated API against
`@boardgamers/protocol` **0.11.0** types and runtime schemas; there is no separate
JavaScript scoring implementation.

## Viewer 0.1.224 / engine 0.4.157 — revision 5, 45 definitions

Adds three independent, inclusive final-state milestones:

| Name | ID | Condition | Reused asset key |
| --- | --- | --- | --- |
| 60 Points | `score-60` | `player.victory_points(game) >= 60` | `achievement-score-40` |
| Seven Objectives | `seven-objectives` | `completed_objectives.len() >= 7` | `achievement-three-objectives` |
| Thirty Advances | `thirty-advances` | `advances.len() >= 30` | `achievement-twenty-advances` |

The score is the canonical total VP, including city pieces, regular and special
advances, completed objectives and objective bonus VP, wonders, event VP and
captured leaders (and the engine's applicable scoring adjustments). It is not a
resource count. Objectives use the same completed-record list as the three- and
five-objective tiers: instant and status-phase completions count alike; held cards,
printed options, bonus VP and lifetime totals do not. Advances follow the existing
Twenty Advances tier: regular advances currently in the set, regardless of how
acquired, excluding civilization special advances (including automatically unlocked
ones). The prior catalog has no 50-point or 25-advance tier.

All three require an authoritative `Finished` save and an eligible non-dropped human
civilization seat, but not a win. Existing platform eligibility responsibilities
still apply. The previous 42 definitions and their conditions are unchanged.
There are still 41 unique achievement asset keys (58 named artwork assets overall);
no artwork bytes, assets or localization metadata change. The wrapper versions and
artwork package-lock integrity hash are updated for the matching release.
Publish only after matching main-branch Check and Web viewer CI pass, using the
frozen tested engine/viewer. The asynchronous engine installer owns activation:
verify public and admin revision 5 with exactly 45 definitions before reporting
completion, including equality of the previous 42 definitions and all 58 named
asset bytes. No historical backfill or live-game mutation is performed; awards
apply to future natural eligible completions. Generated artifacts remain ignored.

## Revision 4 — 42 definitions

Adds **Five Objectives** (`five-objectives`): “Complete 5 objectives in a single
game.” The condition is `completed_objectives.len() >= 5` in one eligible finished
save, additive to the existing three-objective tier and independent of winning.
The engine records one completed objective when its card is completed and discarded;
it does not count both printed options, cards in hand, objective opportunities,
objective victory points, or lifetime totals. Instant and status-phase completions
feed the same list; platform clock modes do not change this evaluator.

Reuses `achievement-three-objectives` as objective-category artwork (the existing
three-tablet image), with no new named asset or upload. The faction artwork manifest only refreshes
its package-lock integrity hash for the wrapper version bump.
Revision-2 faction and revision-3 nonfaction artwork inventories remain intact;
multiple definitions may reference the same asset. Achievement metadata is excluded
from viewer localization extraction, so no changes to the 17 viewer locales are
needed. All prior definitions, award conditions and eligibility gates are unchanged.
The matching engine upload activates the installer-owned achievement manifest;
a generic metadata PUT cannot set it. Release verification must confirm revision 4
and the exact 42-definition catalog through both authenticated and public GETs,
not merely confirm that the engine archive uploaded. Existing 41 entries must
remain identical. Publication uses the normal viewer/engine publisher after source
push and green CI. No historical backfill or live-game edits are performed; the
platform captures achievements on future natural eligible completions.

## Previously delivered revision 2 catalog — 41 definitions

Revision 2 removes `four-cities` and replaces it with a distinct `seven-cities` ID.
There is no migration, historical backfill, or repurposing of the old ID.

Each length has `complete-<length>` and `win-<length>` achievements:

| ID suffix | Explicit length | Rules |
| --- | --- | --- |
| `shorter` | Shorter | 4 ages, two starting settlers, two status-phase advances |
| `standard` | Standard | 6 ages; also the legacy/default option |
| `roll-to-end` | Roll to End | 6–7 ages, end rolls from age 6 round 2 |
| `wonderful-ending` | Wonderful Ending | 6–7 ages, qualifying wonder ends after its round |
| `epic` | Epic | 8 ages |

Additional feats are evaluated independently of victory:

| ID | Final-state evidence |
| --- | --- |
| `score-30` | At least 30 victory points using engine scoring |
| `score-40` | At least 40 victory points using engine scoring |
| `seven-cities` | At least seven cities currently controlled, not cities founded over time |
| `twenty-advances` | At least 20 regular advances (not special advances) |
| `three-objectives` | At least three completed objective records |
| `two-wonders-built` | At least two recorded wonders built, not merely owned |
| `captured-leader` | At least one recorded captured leader |

| `complete-builder` | Complete a game with `options.variant = Builder`, regardless of winning |

Seven cities is the normal legal capacity (`CITY_LIMIT = 7`); permanently destroyed
city centers reduce that capacity. The badge checks the final owned-city count,
not capacity or a historical counter. Builder disables military attacks between
human players, but retains ordinary game endings and scoring. Its completion badge
is additive to the relevant length-completion badge; it is not a Builder win badge.

### Civilization wins and symbols

Each of the 15 actual playable civilizations has a strict-win achievement. Full
score ties do not count. The match uses the canonical final civilization name:

| Civilization | Achievement ID | Asset key |
| --- | --- | --- |
| Rome | `win-rome` | `achievement-faction-rome` |
| Greece | `win-greece` | `achievement-faction-greece` |
| China | `win-china` | `achievement-faction-china` |
| Vikings | `win-vikings` | `achievement-faction-vikings` |
| Babylonia | `win-babylonia` | `achievement-faction-babylonia` |
| India | `win-india` | `achievement-faction-india` |
| Egypt | `win-egypt` | `achievement-faction-egypt` |
| Phoenicia | `win-phoenicia` | `achievement-faction-phoenicia` |
| Maya | `win-maya` | `achievement-faction-maya` |
| Persia | `win-persia` | `achievement-faction-persia` |
| Japan | `win-japan` | `achievement-faction-japan` |
| Huns | `win-huns` | `achievement-faction-huns` |
| Celts | `win-celts` | `achievement-faction-celts` |
| Aztecs | `win-aztecs` | `achievement-faction-aztecs` |
| Carthage | `win-carthage` | `achievement-faction-carthage` |

Barbarians, Pirates and the Choose Civilization placeholder are not playable
faction achievements. Icons use the viewer's existing civilization symbols, not
new logos. The manifest references named assets, not viewer-bundle URLs. After
independent code/artwork approval, upload the reviewed images through the named
asset admin API and verify public durable `achievements/clash/<key>/<hash>/...`
URLs **before** activating revision 2. Do not change bucket policy in this task;
report inaccessible artwork as a publication blocker.

### Individual wonder build records

All eight supported real wonders also have an individual feat, independent of
victory, using the exact engine `wonders_built` membership. `Hidden` is a redaction
placeholder and has no achievement. Names use the viewer's existing Monumental
Edition wording; saved enum IDs are unchanged.

| Saved wonder ID | Display name | Achievement ID |
| --- | --- | --- |
| `Colosseum` | Great Arena | `built-great-arena` |
| `Pyramids` | Great Pyramids | `built-great-pyramids` |
| `GreatGardens` | Great Gardens | `built-great-gardens` |
| `GreatLibrary` | Great Library | `built-great-library` |
| `GreatLighthouse` | Great Lighthouse | `built-great-lighthouse` |
| `GreatMausoleum` | Great Mausoleum | `built-great-mausoleum` |
| `GreatStatue` | Great Statue | `built-great-statue` |
| `GreatWall` | Great Wall | `built-great-wall` |

Each means **finish with that wonder recorded as built by your civilization**.
Ownership is not substituted for building: a captor without a built record does
not qualify, while the original builder can still qualify after capture if its
record survives. The two-wonder badge remains a separate requirement for at least
two surviving built records. Individual wonder badges use generic icon fallback;
no additional artwork or artwork provenance claims are introduced.

All wonder build feats deliberately use the **final surviving `wonders_built` record**,
not a lifetime construction count: `destroy_wonder` removes a destroyed wonder
from that record for every player. Two wonders built earlier do not qualify if
fewer than two remain recorded at completion. Merely capturing/owning a wonder
never substitutes for a built record. This is covered by a destruction regression.

Faction definitions include official optional `{type: "asset", key: ...}` icons.
Other definitions omit icons and use the platform's generic fallback.
IDs are permanent identities and must not be repurposed. This catalog is BGS
platform metadata, not text rendered by the game viewer, so the viewer localization
extractor excludes it. Protocol 0.11.0 also requires `openChat` and `undo` commands;
the isolated preview/tutorial adapters explicitly report those unsupported.

## Eligibility and outcome

Only `Finished` authoritative saves are evaluated. Secret-stripped snapshots,
ongoing games, barbarian/pirate players, and withdrawn seats earn nothing.
Original seat indexes are preserved. An all-withdrawn game returns no results.
The engine's score comparison includes withdrawal precedence and every existing
scoring tiebreaker. An unresolved shared first place is a draw, not a win; other
seats lose. Completion and other feats do not require a win. Withdrawal victories
follow the engine's terminal state rather than inventing an age minimum.

Clash has no serialized cancellation flag or `cancelled` hook. Platform-side
cancellation, practice/analysis eligibility, account identity, and bot-account
filtering remain BGS responsibilities; the evaluator cannot authenticate a save
or detect metadata absent from it. `is_human()` distinguishes playable civilization
seats from NPC factions, not account ownership. Do not run these hooks on arbitrary
client submissions or use their results as proof of platform eligibility.

Evaluation does not advance play, consume randomness, change saves, or expose
hands, hidden cards, seeds, history, or account identifiers. Civilization wins use
the final canonical civilization and engine outcome; no inferred combat history
or unrecorded historical civilization claims are awarded.

## Verification

```sh
cargo test -p server --lib achievements
npm --prefix web-client ci --ignore-scripts
npm --prefix web-client run build:bridge
cd web-client
node --experimental-strip-types --test src/achievements.test.ts
npm run check
```

Generated WASM/JS build products remain ignored and are not committed.
