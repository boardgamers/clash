# Explorers eligibility (A21 / A22)

Independently checked both printed faces in the [English action-deck scan](https://cdn.steamusercontent.com/ugc/1781731512046582765/54C4F5AA6E9F0654E9E9379CD3D4BEECF8F7293C/)
identified by `card-text-audit.json` (Steam Workshop 2651868219, Monumental Edition).
This is a gameplay-text check, not an inference from that audit's title verification.

Both faces say (culture-token icon rendered in brackets):

> Pay 1 [culture token] to play.
>
> As an Action, do one or both of the following:
> 1. Reveal an unrevealed region adjacent to a region with one of your cities.
> 2. Place a free Settler in one of your cities.

The 9586 × 10000 texture has eight columns and six rows. A21 is column 5,
row 3; A22 is column 4, row 5 (one-based). Their tactics are Routing and
Prepared Defenses respectively.

The two benefits are independent: a city and a Settler in supply suffice even
without an eligible unrevealed region. An eligible region still suffices when
Settlers are exhausted. Neither benefit available remains ineligible. Existing
optional resolution, action/culture costs, Shogunate handling, and supply-based
unit gain without Recruit/city activation are unchanged.
