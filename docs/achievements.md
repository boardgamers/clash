# Clash of Cultures achievements

Delivered engine release: `@boardgamers/clash-server` **0.4.150**, BGS `clash/1`
(Clash of Cultures: Monumental edition). Engine-only release; the hosted viewer
remains 0.1.217. See `web-client/release/engine-package.json` for package metadata
and [the delivery receipt](../web-client/docs/achievements-0.4.150-receipt.md) for
the exact reviewed artifact, source commit and activation evidence.

The native Rust evaluator is the single source of truth. The WASM package exports
`achievementDefinitions()` and `achievements(serializedData)` directly, alongside
its existing engine hooks. The web-client tests check the generated API against
`@boardgamers/protocol` **0.11.0** types and runtime schemas; there is no separate
JavaScript scoring implementation.

## Revision 1 catalog

Each length has `complete-<length>` and `win-<length>` achievements:

| ID suffix | Explicit length | Rules |
| --- | --- | --- |
| `shorter` | Shorter | 4 ages, two starting settlers, two status-phase advances |
| `standard` | Standard (Normal) | 6 ages; also the legacy/default option |
| `roll-to-end` | Roll to End | 6–7 ages, end rolls from age 6 round 2 |
| `wonderful-ending` | Wonderful Ending | 6–7 ages, qualifying wonder ends after its round |
| `epic` | Epic | 8 ages |

Additional feats are evaluated independently of victory:

| ID | Final-state evidence |
| --- | --- |
| `score-30` | At least 30 victory points using engine scoring |
| `score-40` | At least 40 victory points using engine scoring |
| `four-cities` | At least four cities currently controlled |
| `twenty-advances` | At least 20 regular advances (not special advances) |
| `three-objectives` | At least three completed objective records |
| `two-wonders-built` | At least two recorded wonders built, not merely owned |
| `captured-leader` | At least one recorded captured leader |

The wonder feat deliberately uses the **final surviving `wonders_built` record**,
not a lifetime construction count: `destroy_wonder` removes a destroyed wonder
from that record for every player. Two wonders built earlier do not qualify if
fewer than two remain recorded at completion. Merely capturing/owning a wonder
never substitutes for a built record. This is covered by a destruction regression.

Icons are omitted intentionally: the platform's generic fallback is sufficient.
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
hands, hidden cards, seeds, history, or account identifiers. No inferred combat
history or civilization-specific claims are awarded.

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
