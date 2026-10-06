# Clash of Cultures tutorial curriculum

This proposal comes from a complete read of the available CoC game chats and player feedback on 6 October 2026: 12 games, 203 chat records (164 human text messages), 47 private feedback threads and 93 replies. There were no matching public discussions or public feedback records. Private messages and player names are deliberately absent from this document.

The recurring problem is knowing _which step happens next_, not remembering every rule. Each chapter should start from a small deterministic position, ask the learner to perform a real action, show its consequence, and finish with one short decision. Use the production engine to validate actions. An invalid or unrelated action should leave the position unchanged and explain the required step. Chapters must be independent, resumable and restartable through the BGS tutorial protocol.

Metadata remains in English for translation through BGS admin. Tutorial prose belongs in the viewer's per-language catalogs, with the same glossary as the game UI. Do not register unfinished chapters in gameinfo.

## Suggested order

### 1. Your first turn (`first-turn`, basics, 5 minutes)

**Problem:** Players miss whose turn it is, the three-action limit, or confuse choosing a second movement group with taking another action.

**Position:** Two civilizations; learner is active with a size-1 neutral city, an adjacent settler and enough resources for one advance. No imminent incident.

**Steps:** Identify the active civilization and remaining actions; collect once; inspect the resulting activation marker; research one advance; end the turn with one action left. Explain that spending all three actions is optional, and that passing commits the turn.

**Check:** Selecting an opponent's piece does not grant an action. A move belonging to the opponent must be refused. The action count must fall once per actual action, not once per dialog click.

### 2. A productive city (`city-economy`, basics, 7 minutes)

**Problem:** Happy collection, capacity, terrain yields and storage overflow cause repeated questions.

**Position:** Happy size-2 city beside fertile land, forest and mountains; two food already stored, without Storage. Include one collection option with a resource substitution.

**Steps:** Inspect capacity (3); choose three legal yields; change one allocation without losing the others; notice the food storage warning; select a useful non-overflowing collection; compare the result with a neutral city. Inspect Taxes as an alternative action, rather than an extra resource from collection.

**Check:** Capacity limits and tile yields come from the engine. Ball Courts changes must preserve valid allocation choices. The lesson must not rely on silently discarding overflow.

### 3. Activation and happiness (`activation-happiness`, basics, 6 minutes)

**Problem:** City activation and happiness are mistaken for the same mechanic; players miss the global multi-city happiness action.

**Position:** Two size-1 cities, one neutral and already activated, one angry and unused; enough mood tokens to improve both.

**Steps:** Predict the first city's mood after a second activation; increase happiness in both cities in one action; verify happiness did not activate either city; collect from the city previously angry; inspect the new mood and activation state. Explain the once-while-angry restriction and why an angry city cannot construct.

**Check:** Improving two cities spends one action, with the cost calculated for each selected city and each mood step. No activation marker changes during happiness.

### 4. Research with a plan (`research-paths`, basics, 7 minutes)

**Problem:** Research payments are confused with an ideas-only cost; players miss free advances and civilization prerequisites.

**Position:** Learner can afford research with food and ideas, is near Math, and has an available civilization advance. Keep Free Education and Priesthood for an optional final panel.

**Steps:** Open the research tree; pay with food and ideas; follow the prerequisites to Math; inspect the “Free with Math” hint on Engineering/Roads; claim the resulting free advance; inspect civilization-specific research and the other civilizations below the regular tree. Identify the remaining event markers.

**Check:** The lesson reads current engine costs and dependencies. It must distinguish researching for no resource cost from spending no action. Explain Free Education's prerequisite separately from normal research payment, and Priesthood's once-per-turn use.

### 5. Move, explore, found (`movement-founding`, expansion, 8 minutes)

**Problem:** Founding is confused with finishing a movement group; exploration is assumed to reveal a single hex.

**Position:** Two settler groups, safe adjacent land and an unexplored region; army units are visible but Tactics is absent.

**Steps:** Start Move; move the first settler to the forest; explore with the second settler within the same Move action and choose the revealed region’s orientation; after movement finishes, found a city with the first settler using a separate action. The whole sequence spends two actions, leaving one action available.

**Check:** Movement spends one action for up to three groups. A unit cannot join two groups within that action. Tactics restrictions and founding legality use the real engine. Failed destinations do not change the state.

### 6. Ships and passengers (`sea-transport`, expansion, 7 minutes)

**Problem:** Boarding, sailing and landing allowances are difficult to infer from the UI.

**Position:** One ship beside a settler, a connected sea route and an unexplored region beyond the starting city.

**Steps:** Board and sail in one Move action; check why a newly boarded passenger needs another Move action to land; disembark into unexplored C4 and choose the region’s orientation.

**Check:** Boarding and sailing spend one action; landing and exploring spend another. The settler ends at C4 without a carrier, and the region is revealed. Rejected moves remain atomic.

### 7. Cultural influence, one decision at a time (`cultural-influence`, interaction, 8 minutes)

**Problem:** Range, payment, the roll and an optional boost look like unrelated dialogs; building color is mistaken for city ownership.

**Position:** A size-2 source city, an enemy building three spaces away, sufficient culture tokens, and a deterministic roll of 3.

**Steps:** Select the source and target; inspect the one-token range extension _before_ rolling; confirm payment; roll; inspect the two-token shortfall; choose whether to pay for success; inspect the changed building color, point and unchanged city owner. Finish with the consequence of declining the boost and the one-success-per-turn limit.

**Check:** Fix the tutorial seed, rather than replacing engine dice. Range payment and success boost remain distinct. Neither declining nor an unaffordable boost should consume resources accidentally. Show the running cost and next step throughout.

### 8. Incidents and hostile units (`incidents-hostiles`, interaction, 8 minutes)

**Problem:** Hostile placement guides are mistaken for player actions; Heavy Flood and Myths decisions are misunderstood.

**Position:** One incident is imminent; separate saved subpositions demonstrate Heavy Flood and a pirate/barbarian incident.

**Steps:** Trigger and read the incident; inspect every affected city for Heavy Flood; choose protection where permitted; verify the remaining cities' results; follow a barbarian placement/movement guide; identify horned helmets and a barbarian city's helmet flag; identify pirate flags and their sea effects. Explain barren and exhausted terrain separately.

**Check:** Use the actual incident's target list; do not imply all incidents select one city. Protection must use the real Myths rules and payment. Empty incident phases should not appear as substantive replay actions.

### 9. Cards, leaders and free actions (`cards-leaders`, mastery, 7 minutes)

**Problem:** “Free action” is read as “free payment”; draft, recruit and card play are conflated.

**Position:** Great Prophet available with a legal Temple construction, one recruitable leader and a Shogunate example in a separate checkpoint.

**Steps:** Read Great Prophet's conditions and payment; build without spending a normal action, while paying the listed cost; inspect leader recruitment and movement requirements; distinguish Shogunate card drafting from playing a card and its separate allowance. Show Draft as military conscription of one unit, not civilization selection.

**Check:** Free actions still pay their specified resource costs. Ability counters reset at the correct turn boundary. Do not use a formerly broken interaction as a tutorial rule.

### 10. Wonders and ownership (`wonders-ownership`, mastery, 7 minutes)

**Problem:** Building a wonder, owning it after capture, and triggering its ability are confused; fractional scoring surprises players.

**Position:** Separate checkpoints with Great Lighthouse, Great Mausoleum, Great Explorer and a captured Pyramid.

**Steps:** Activate a Lighthouse city and decide on its optional ship; verify this is a follow-up to that activation, not another free city activation; inspect Mausoleum's owner restriction; meet Great Explorer's exploration conditions and inspect its optional benefit; compare Pyramid's builder points with its current owner's points. Inspect the same score in the viewer and BGS sidebar.

**Check:** Validate each trigger against current engine rules. The Pyramid builder receives 5.1 points; ownership alone gives 0 wonder points. A failed ability action cannot partially update the game.

### 11. Objectives and the end of an age (`objectives-ages`, mastery, 6 minutes)

**Problem:** Players expect generic battle wins to satisfy objectives with extra conditions, or try to claim end-of-age objectives early.

**Position:** One during-play objective and one end-of-age objective, with progress one step from completion.

**Steps:** Read the actual condition; perform the qualifying action; claim the eligible objective; advance to age-end checks; claim the age-end objective; observe free research and new cards. Contrast the final age, which ends before those rewards.

**Check:** Match objective IDs and triggers, not translated name text. A battle example must satisfy every printed condition. Show completion progress rather than treating any victory as sufficient.

### 12. Read, review and recover (`platform-tools`, reference, 5 minutes)

**Problem:** Undo, visual replay and analysis are confused; useful rules links and preferences are hidden.

**Position:** A short existing example history with one information reveal and two harmless actions, loaded as an isolated tutorial fixture.

**Steps:** Click an advance or card in the journal; locate a coordinate on the map; step through the last turn's actions; observe that empty phase changes are skipped; inspect the undo boundary at an information reveal; enter analysis and return to the live position. Visit available-actions filtering, colorblind display and map-orientation preferences in platform settings.

**Check:** Tutorial analysis never writes to a live game. The replay example uses the current action log format. A failed replay must leave the original position intact. Explain that “available actions only” filters choices; it does not execute or skip decisions.

## Delivery and acceptance

Implement chapters in this order: 1–5 and 7 first, then 6 and 8–12. These address the most frequent onboarding questions without requiring every optional rule on day one. Keep each chapter around five to eight minutes and offer a skip-to-reference route for experienced players.

Each chapter needs a current engine-generated fixture, deterministic scripted opponent responses, constrained real actions, one failure-path test, a complete successful walkthrough, a restart/restore test and screenshots in English plus at least one non-Latin locale. Save action history rather than trusting serialized tutorial progress. Keep stable chapter IDs, and increment the chapter version when an engine/rule change invalidates stored history.

One feedback item remains open: selecting a hex sometimes opens movement and sometimes city details. That is a UI consistency problem. Resolve it in the viewer; a tutorial should explain a consistent interaction, not teach players to anticipate a surprising one.

## Implemented release scope

The release registers all 12 independent chapters in five sections: basics, expansion, interaction, mastery and platform tools. Each chapter has a small production-engine position, constrained legal moves, explanations and/or checks, plus validated save, restart and failure behavior. The longer scenarios above remain curriculum ideas; they do not all occur in the initial chapters.

| Chapter                        | Interactive exercise in this release                                                                              |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| Your first turn                | Collect, research Storage, end the turn with an action unused.                                                    |
| A productive city              | Collect from a happy size-2 city; check capacity and overflow.                                                    |
| Activation and happiness       | Improve two cities together, then collect and compare activation counts.                                          |
| Research with a plan           | Research Math and claim Engineering without a resource payment.                                                   |
| Move, explore, found           | Move a settler, finish movement, found a city, then explore and orient a region.                                  |
| Ships and passengers           | Board, sail, then disembark to explore a new region in the next action.                                                                        |
| Cultural influence             | Select a Temple, pay extra range, see the roll, pay the boost, and compare building and city ownership.           |
| Incidents and hostile factions | Trigger Heavy Flood, protect with Myths, and resolve the scripted opponent's city choice; inspect hostile guides. |
| Cards and leaders              | Recruit Caesar and distinguish a free action from a free payment through a check.                                 |
| Wonders and ownership          | Activate a Lighthouse city and accept its optional ship; check Pyramid builder scoring.                           |
| Objectives and ages            | Recruit twice and claim Draft; check when final-age rewards stop.                                                 |
| Platform tools                 | Undo a harmless research action; distinguish analysis and explain the platform preferences.                       |

Tutorial game steps use the normal board controls. The sidebar provides a control path and highlights the relevant controls; only quiz answers have sidebar buttons. Legal collection choices, research payments and city selection order are accepted and validated by the engine. Unrelated actions are rejected without changing the position. The 12 walkthrough tests also restore saved history and restart, and semantic assertions verify the key rules effects.
