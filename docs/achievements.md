# Clash of Cultures achievements

Delivered engine release: `@boardgamers/clash-server` **0.4.151**, BGS `clash/1`
(Clash of Cultures: Monumental edition). Revision 2 and all 15 faction assets were
independently reviewed, published and verified. The hosted viewer remains 0.1.217;
no viewer release was made. See `web-client/release/engine-package.json` and the
[revision-2 delivery receipt](../web-client/docs/achievements-0.4.151-receipt.md).
The [revision-1 receipt](../web-client/docs/achievements-0.4.150-receipt.md)
records the previously delivered 0.4.150 artifact.

The native Rust evaluator is the single source of truth. The WASM package exports
`achievementDefinitions()` and `achievements(serializedData)` directly, alongside
its existing engine hooks. The web-client tests check the generated API against
`@boardgamers/protocol` **0.11.0** types and runtime schemas; there is no separate
JavaScript scoring implementation.

## Revision 2 catalog — 41 definitions

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
