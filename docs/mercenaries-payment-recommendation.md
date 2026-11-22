# Mercenaries payment recommendations

This is a product preference, not a rules change. Eligible Carthaginian
Mercenaries recruitment costs carry an explicit `Mercenaries` recommendation
policy. Other payments retain their existing gold-first recommendation.
Missing metadata deserializes as the native policy, and native metadata is
omitted from serialization.

The scoped policy minimizes affordable total units paid (including mood,
culture and captives), then gold, then uses the existing deterministic native
search order. The regular printed cost stays `default`: it still defines the
same legality and mandatory Warbeasts replacement. Manual regular payments
remain valid. The picker uses the same ordering, without moving empty/decline
slots or changing rewards.

For Elephant + Ship without Market, competing food/wood/gold stock now chooses
1 food + 2 gold over 1 food + 1 gold + 2 wood. With mood available it prefers
1 food + 1 gold + 1 mood. Ship alone prefers 1 gold over 2 wood. Civil Liberties
makes Draft cost two items, so a rich Ship payment keeps native 2 wood rather
than gratuitously spending two tokens.

Native conversion-order ties are deliberately not an additional token-conservation
objective: with abundant culture the current search can choose 2 culture +
1 mood rather than 1 food + 1 culture + 1 mood. Both have three items and zero
gold. This preserves native deterministic conversion traversal rather than
introducing a global (or hidden third) resource preference.

Recommendation uses binary feasibility searches for quantity and gold, with
quantity passed through alternative and separate-fee branches into the existing
conversion bounds. No payment enumeration cap is used to find the recommendation.
The scoped gold search starts at zero, since Warbeasts' printed gold can legally
be replaced with culture; the generic printed-gold floor is unchanged.

Web selected-payment queries, action payloads, per-unit previews and the legacy
payment UI already consume the shared engine helper. No UI-only override or new
interaction is needed. Tests use synthetic/local states only; no game backfill.

## Release 0.1.236 / engine 0.4.167

Publish the matching bridge and engine together, after the exact source commit's
Check and Web viewer workflows pass. The Web viewer workflow includes the
Carthage recruitment, native research/manual-gold, and wonder-payment performance
regressions. Native tests include the small-input exhaustive conversion oracle,
separate-fee quantity budgeting, legacy metadata defaults and omitted native
serialization. The WASM suite verifies quantity/gold ordering and still executes
all legal offered payments, including an explicitly selected regular Ship cost.

Desktop (1400px) and mobile (390px) checks cover automatic discounted recruitment,
manual regular recruitment, and the disclosed abundant-token tie. Other research
recommendations and manual gold choices remain unchanged. No achievement catalog,
artwork, gallery, preferences, settings or Standard-option changes are included;
the artwork source manifest's lockfile pin changes only for the viewer version.

