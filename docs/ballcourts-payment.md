# Ballcourts and Great Arena payment

Verified against the [official Monumental Edition rulebook](https://wizkids.com/posters/repository/wizkids/Clash-of-Cultures---Rulebook---WEB---2021-02-05-compressed.pdf):

- Page 28, Great Arena: “allows you to spend [culture] as [mood] and vice versa—except when spending [culture] to Construct a [Wonder].” It further specifies that when “spending, discarding, or otherwise using the tokens, they are interchangeable.” Brackets transcribe the printed icons.
- Page 34, Maya / Ballcourts: “When you Collect Resources or Recruit Units, you may pay 1 [mood] to treat the city as 1 size larger, allowing you to Collect 1 more Resource or Recruit 1 more Unit (of course still paying its cost).” Its separate once-per-turn Spirituality discount remains unchanged. This increases capacity, not available actions or activation permissions.

## Bug and fix — viewer 0.1.229 / engine 0.4.161

Collection checked the raw mood balance in both collection validation and the web preview, then directly debited one mood during execution. All three paths bypassed the engine's general payment conversions. Recruitment already included the fee in its `PaymentOptions` and supported Great Arena correctly.

Collection keeps its original immediate payment flow: the Ballcourts fee is automatically charged with the shared `PaymentOptions::first_valid_payment` conversion helper, preferring mood as before and using culture when mood is unavailable and Great Arena allows it. There is no new payment confirmation, including when both tokens are available. Recruitment's existing explicit payment selection is unchanged. The combined cost is used by the preview and authoritative action boundary to check affordability before collection, so collected resources cannot fund either fee.

The preview subtracts the recommended action payment when predicting resulting resources/waste. The affordability check retains any modified collection action's original cost and adds the Ballcourts fee, so one token cannot fund both costs. Free Economy keeps its existing one-token payment request; execution then charges the separate Ballcourts fee exactly once from the remaining tokens. Standard city activation, capacity, terrain and per-tile validation still applies. GUI toggles depend on the existing Ballcourts/non-Angry eligibility flag, not a separate mood balance check.

Regression coverage: native `ballcourts_payment_tests`, plus WASM-backed `remaining-civilizations.test.ts` and desktop/mobile built-viewer `node web-client/scripts/ballcourts-payment-smoke.mjs` (ordinary collection and Free Economy, culture-only payment, no added decision); the existing Spirituality, Terracing, collection-capacity and controller tests remain relevant. The engine's Great Arena conversion listener and wonder-construction exception are not changed.

The regression was also run against the original three engine files: recruitment completed successfully using culture, then collection failed with `Ballcourts costs 1 mood token`. With the fix, both actions complete and deduct exactly one culture token. Native coverage also exercises Free Economy + Ballcourts: one culture is insufficient; two pay both fees while retaining Free Economy's zero action-point cost.

A narrow scan of other raw mood checks/direct resource debits did not identify another matching Ballcourts-style mood fee bypass; Calendar, Free Economy, happiness and recruitment already use payment options. No broader payment-rule changes were made.

## Historical replay verification

The earlier local prompt implementation was **not history-compatible**: it left an old immediate collection waiting for a new payment response. Replaying the frozen fixture below then failed at move 12 with `action should be a response: Playing(Collect(...))`. This was reproduced with the actual compiled prompt implementation, not inferred from code. The prompt was removed rather than introducing a history boundary or migration. The existing asynchronous-combat boundary pattern was inspected but is unnecessary when retaining this original action sequence.

`server/tests/fixtures/legacy_ballcourts_history.json` is a frozen synthetic, legal history generated entirely by production **0.4.160**, without edits to resources, advances, cities, or logs. Starting from seed `ballcourts-history`, Maya researches Arts, collects twice, ends both players' turns, researches Sports, collects with Ballcourts (automatically spending its one mood), then makes another ordinary collection. The expected JSON records production states both immediately after Ballcourts and after the following action. Full and partial replay, a new branch, undo/redo of both collections, and branch replay match production. No payment response is inserted or removed.

Production artifact source:

- Package: `@boardgamers/clash-server` version `0.4.160` (source release commit `8b3b81e7`).
- URL: <https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/engine/fadd09ac21e7b5dd/boardgamers-clash-server-0.4.160.tgz>
- Archive SHA-256: `fadd09ac21e7b5dd659950a3ab266ae5a0fb08dbe1bbec119ce429393690da20`.
- `server_bg.wasm` SHA-256: `3be580e4bf734620e2cc827f4c871b2ae6fbbb86115abe3b9574df7d44e93698`.

The explicit maintenance generator is `scripts/generate-legacy-ballcourts-history.mjs /absolute/path/to/production/package/server.js`; it checks the package version and verifies both snapshots with production replay. Normal tests only read the frozen files; they never regenerate expectations with the current engine. Run `node --experimental-strip-types --test web-client/src/ballcourts-history-engine.test.ts` after building the bridge. Set `CLASH_TEST_ENGINE=/absolute/path/to/production/package/server.js` to run the same three compatibility tests against production. Both production and the corrected local WASM pass. The original culture-only bug still fails on production, while the local culture-only collection/recruitment tests pass.

Native coverage also verifies mixed-token Free Economy payments (either original token choice), two mood, two culture, insufficient combined tokens, automatic mood preference, no double charge, and unchanged action/activation counts. The matching release is viewer 0.1.229 / engine 0.4.161.
