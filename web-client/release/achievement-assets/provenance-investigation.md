# Existing PNG provenance investigation (not clearance)

The actual existing PNG masks remain the inputs. Their hashes are pinned in
`../achievement-assets.json`. No substitute illustrations were introduced.

SVG Repo item-page metadata identifies the candidates below, but the original
SVG image requests were blocked or failed to render during verification. A page
name or license label does not prove its image matches a local PNG. **None of
these candidate matches establishes a license for the local PNGs.**

| Existing symbol | Candidate item pages | Observed page license/provider |
| --- | --- | --- |
| Greece / temple-building-with-columns | <https://www.svgrepo.com/svg/89426/temple-building-with-columns> | CC0; uploader SVG Repo |
| Rome / colosseum-rome | <https://www.svgrepo.com/svg/184166/colosseum-rome>, <https://www.svgrepo.com/svg/184188/colosseum-rome>, <https://www.svgrepo.com/svg/220508/colosseum-rome>, <https://www.svgrepo.com/svg/301824/colosseum-rome>, <https://www.svgrepo.com/svg/301851/colosseum-rome> | CC0; uploader SVG Repo |
| China / great-wall-of-china-chinese | <https://www.svgrepo.com/svg/220522/great-wall-of-china-chinese>, <https://www.svgrepo.com/svg/229211/great-wall-of-china-chinese> | CC0; uploader SVG Repo |
| Vikings / viking-ship | <https://www.svgrepo.com/svg/101379/viking-ship>, <https://www.svgrepo.com/svg/101393/viking-ship>, <https://www.svgrepo.com/svg/157471/viking-ship>, <https://www.svgrepo.com/svg/50060/viking-ship>, <https://www.svgrepo.com/svg/61008/viking-ship>, <https://www.svgrepo.com/svg/86592/viking-ship>, <https://www.svgrepo.com/svg/92309/viking-ship>, <https://www.svgrepo.com/svg/95191/viking-ship> | CC0; uploader SVG Repo |
| Other same-slug Viking candidates | <https://www.svgrepo.com/svg/480728/viking-ship>, <https://www.svgrepo.com/svg/480817/viking-ship> | PD; Icooon Mono |

SVG Repo's license explanation: <https://www.svgrepo.com/page/licensing/>.
CC0 reference: <https://creativecommons.org/publicdomain/zero/1.0/>.
“Uploader SVG Repo” is not a claim that SVG Repo was the original individual artist.

Local silhouettes to match against actual licensed originals:

- Rome: Colosseum facade with tall curved left wall descending to low right wall,
  three tiers of cutout arches, wide solid base.
- Greece: triangular pediment, three columns with separate capitals, stepped base.
- China: winding wall leading from foreground toward upper left, battlement tower
  at upper right with arched opening.
- Vikings: dragon head at left, curled stern right, large filled sail, horizontal
  yard and small flag above it.

The Greek candidate page's license was additionally confirmed via a rendered page
capture, but its main artwork was a broken image. Do not treat that capture as a
successful visual match. Exact correspondence and license terms remain unverified.

## Accepted existing-project reuse basis

The user explicitly accepted reuse of the established project artwork. This is not
an assertion of per-item CC0 or root-MIT coverage. A bounded repository attribution
and import-history check found no known restriction, but no per-image license
record either; importing PR bodies are empty.

Original paths and complete hashes/commits are recorded in the manifest:

- Rome and China: imported by `603be9b8f0a361bebcef69d019c6451d75255eb8`, PR #233 (2025-04-30).
- Greece: imported by `7fc48ab4187c9d8bb874b11ec2581ce1b4a1fc97`, PR #97 (2025-01-02).
- Vikings: imported by `77bfc205eec2b664b24846753e0680ebf6ec9740`, PR #265 (2025-06-09).

`git show <commit>:<sourcePath>` bytes matched the current pinned source hashes.
A read-only download of the production viewer URL recorded in each manifest entry
confirmed that all four exact PNG byte sequences are embedded in that bundle.
`web-client/src/civilization-emblems.ts` uses them for these same factions; the
native client also references the source files. The named-asset registry contains
only a pictogram and screenshots, not reusable faction entries.

The scoped publisher guard accepts these four pinned identities as
`verified-existing-project-asset`, with explicit `per-item-license-unverified`
status and user authorization. Missing metadata is not treated as a prohibition.
Other assets cannot use this path without a separately reviewed code change.
