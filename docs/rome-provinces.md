# Rome: Provinces (viewer 0.1.241 / engine 0.4.172)

The primary [Rome board, image 9488277](https://boardgamegeek.com/image/9488277)
was visually verified during investigation:

> After you capture a city, make it [Neutral] as a last step (or [Happy] if you pay 1 [Culture]).

Both benefits previously lived in the optional-payment response. The shared
payment helper suppresses unaffordable requests, so a player without a legal
one-culture payment never received the mandatory Neutral benefit. Leadership
25 correctly spends one culture for one action; it only exposed this bug.

Provinces now sets a retained captured city Neutral at combat-end priority 21,
before offering its optional Happy upgrade. Proconsul remains 22 and Captivi 20.
No global payment semantics changed. PaymentOptions still permits Great Arena's
mood conversion; zero available culture and mood does not create a prompt.
Affordable pending prompts already show the Neutral city. Declining leaves it
Neutral; paying upgrades it Happy exactly once. Legacy pending prompts still
resolve, without downgrading an already Happy city on decline. Missing or
no-longer-owned cities receive no mood mutation and any submitted payment is
refunded. Automatically razed captures generate no Provinces request.

## History compatibility

Correcting historical Angry captures could change later happiness costs and
legal actions. A separate `provinces_from` action boundary therefore follows the
existing independent rule-boundary pattern. New games use zero; old live saves
record their first new action. Historical replay without the field uses old
capture behavior throughout its retained history, then enables the correction
for new actions. Partial replay and branches clamp to the retained cursor plus
one; undo/redo and client prediction preserve the boundary. All save/replay
serialization paths carry it. Existing completed Angry cities are **not** repaired
on load, including the reported C4. This is a code correction, not a live-save edit.

## Coverage and scope

Native tests cover zero-culture capture, old behavior, pending accept/decline,
legacy resume, ownership loss/refund, Happy decline, automatic city-limit razing,
upgrade/undo/redo/branch, and independent full/partial replay serialization using
the existing frozen Architect history. The historical fixture tests boundary
plumbing, not a from-initialization Rome replay. Public-WASM synthetic captures
cover Leadership spending its last culture before China's last/nonlast city,
ordinary zero-culture captures, optional culture payment and Great Arena mood
conversion. No private production fixture is included.

The zero-culture native regression was run against the original Rome listener:
it failed with actual Angry / expected Neutral, then passed with this fix.

Combat-end mood-mutator inspection found later Happy effects for Huns (Attila,
priority -35) and Phoenicia (Deliverance, -20), not a later Roman downgrade.
These civilization-specific effects were left unchanged. The optional-payment
listener API itself is unchanged; other callers were not broadly rewritten.

The shared-helper caller audit also found optional decline logging (Trojan Horse,
Great Artist/temple and warfare) and Celts' selected-army cleanup in response
callbacks. These are not Provinces' mandatory capture benefit; no global change
or speculative fixes were made. Desktop (1400px) and mobile (390px) normal-viewer
smokes verify zero-token absence of a payment prompt, decline, and paid Happy.
The WASM drop-player regression checks deterministic optional decline and leaves
the mandatory Neutral mood and culture balance intact.
