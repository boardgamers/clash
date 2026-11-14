//! Read-only, revisioned achievements derived from authoritative final game state.
use crate::game::{Game, GameLength, GameState, GameVariant};
use crate::victory_points::compare_score;
use crate::wonder::Wonder;
use serde::Serialize;
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Manifest {
    pub revision: u32,
    pub achievements: Vec<Definition>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Icon {
    Asset { key: String },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SeatAchievements {
    pub seat: usize,
    pub earned: Vec<String>,
    pub outcome: String,
}

const LENGTHS: [(GameLength, &str, &str); 5] = [
    (GameLength::Shorter, "shorter", "Shorter"),
    (GameLength::Standard, "standard", "Standard"),
    (GameLength::RollToEnd, "roll-to-end", "Roll to End"),
    (
        GameLength::WonderfulEnding,
        "wonderful-ending",
        "Wonderful Ending",
    ),
    (GameLength::Epic, "epic", "Epic"),
];
const FACTIONS: [(&str, &str); 15] = [
    ("Rome", "rome"),
    ("Greece", "greece"),
    ("China", "china"),
    ("Vikings", "vikings"),
    ("Babylonia", "babylonia"),
    ("India", "india"),
    ("Egypt", "egypt"),
    ("Phoenicia", "phoenicia"),
    ("Maya", "maya"),
    ("Persia", "persia"),
    ("Japan", "japan"),
    ("Huns", "huns"),
    ("Celts", "celts"),
    ("Aztecs", "aztecs"),
    ("Carthage", "carthage"),
];
const WONDERS: [(Wonder, &str, &str); 8] = [
    (Wonder::Colosseum, "built-great-arena", "Great Arena"),
    (Wonder::Pyramids, "built-great-pyramids", "Great Pyramids"),
    (Wonder::GreatGardens, "built-great-gardens", "Great Gardens"),
    (Wonder::GreatLibrary, "built-great-library", "Great Library"),
    (
        Wonder::GreatLighthouse,
        "built-great-lighthouse",
        "Great Lighthouse",
    ),
    (
        Wonder::GreatMausoleum,
        "built-great-mausoleum",
        "Great Mausoleum",
    ),
    (Wonder::GreatStatue, "built-great-statue", "Great Statue"),
    (Wonder::GreatWall, "built-great-wall", "Great Wall"),
];
const FEATS: [(&str, &str, &str); 7] = [
    (
        "score-30",
        "30 Points",
        "Finish a game with at least 30 victory points in the final state.",
    ),
    (
        "score-40",
        "40 Points",
        "Finish a game with at least 40 victory points in the final state.",
    ),
    (
        "seven-cities",
        "Seven Cities",
        "Finish a game owning at least 7 cities in the final state.",
    ),
    (
        "twenty-advances",
        "Twenty Advances",
        "Finish a game with at least 20 regular advances in the final state; special advances do not count.",
    ),
    (
        "three-objectives",
        "Three Objectives",
        "Finish a game with at least 3 completed objectives in the final state.",
    ),
    (
        "two-wonders-built",
        "Two Wonders Built",
        "Finish a game having built at least 2 wonders, as recorded in the final state; merely owning wonders does not count.",
    ),
    (
        "captured-leader",
        "Captured Leader",
        "Finish a game with at least 1 captured leader in the final state.",
    ),
];

/// The stable achievement catalogue. Array order also defines award order.
#[must_use]
pub fn definitions() -> Manifest {
    let mut definitions = Vec::with_capacity(41);
    for (_, id, name) in LENGTHS {
        definitions.push(Definition {
            id: format!("complete-{id}"),
            name: format!("Complete {name}"),
            description: format!("Finish a {name} game without dropping out."),
            icon: Some(Icon::Asset {
                key: format!("achievement-complete-{id}"),
            }),
        });
        definitions.push(Definition {
            id: format!("win-{id}"),
            name: format!("Win {name}"),
            description: format!("Finish a {name} game as the sole highest-ranked non-dropped human seat, including score tiebreakers; tied games do not count as wins."),
            icon: Some(Icon::Asset { key: format!("achievement-win-{id}") }),
        });
    }
    definitions.extend(FEATS.map(|(id, name, description)| Definition {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        icon: Some(Icon::Asset {
            key: format!("achievement-{id}"),
        }),
    }));
    definitions.extend(WONDERS.map(|(_, id, name)| Definition {
        id: id.into(),
        name: format!("{name}: Recorded as Built"),
        description: format!("Finish with {name} recorded as built by your civilization. Destroyed wonders and merely owning captured wonders do not count."),
        icon: Some(Icon::Asset { key: format!("achievement-{id}") }),
    }));
    definitions.push(Definition {
        id: "complete-builder".into(),
        name: "Complete Builder".into(),
        description: "Finish a Builder variant game without dropping out.".into(),
        icon: Some(Icon::Asset {
            key: "achievement-complete-builder".into(),
        }),
    });
    definitions.extend(FACTIONS.map(|(name, slug)| Definition {
        id: format!("win-{slug}"),
        name: format!("Win as {name}"),
        description: format!("Finish a game as {name} as the sole highest-ranked non-dropped human seat, including score tiebreakers; tied games do not count as wins."),
        icon: Some(Icon::Asset { key: format!("achievement-faction-{slug}") }),
    }));
    Manifest {
        revision: 3,
        achievements: definitions,
    }
}

/// Evaluate only authoritative finished games, without modifying game state.
/// Dropped seats and non-human civilizations never receive awards or affect wins.
///
/// # Panics
/// Panics if a game length is added without a corresponding catalog entry.
#[must_use]
pub fn achievements(game: &Game) -> Vec<SeatAchievements> {
    if !matches!(game.state, GameState::Finished) || game.secrets_stripped {
        return vec![];
    }
    let players: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.is_human() && !game.dropped_players.contains(&p.index))
        .collect();
    let (_, length, _) = LENGTHS
        .iter()
        .find(|(length, _, _)| *length == game.options.length)
        .expect("all game lengths have achievement IDs");
    players
        .iter()
        .map(|p| {
            let mut awards = vec![format!("complete-{length}")];
            if players.iter().all(|other| {
                p.index == other.index || compare_score(p, other, game) == Ordering::Greater
            }) {
                awards.push(format!("win-{length}"));
            }
            let score = p.victory_points(game);
            let predicates = [
                score >= 30.0,
                score >= 40.0,
                p.cities.len() >= 7,
                p.advances.len() >= 20,
                p.completed_objectives.len() >= 3,
                p.wonders_built.len() >= 2,
                !p.captured_leaders.is_empty(),
            ];
            for ((id, _, _), met) in FEATS.iter().zip(predicates) {
                if met {
                    awards.push((*id).into());
                }
            }
            for (wonder, id, _) in &WONDERS {
                if p.wonders_built.contains(wonder) {
                    awards.push((*id).into());
                }
            }
            let outcome = if players
                .iter()
                .any(|other| compare_score(p, other, game) == Ordering::Less)
            {
                "loss"
            } else if players.iter().any(|other| {
                p.index != other.index && compare_score(p, other, game) == Ordering::Equal
            }) {
                "draw"
            } else {
                "win"
            };
            if game.options.variant == GameVariant::Builder {
                awards.push("complete-builder".into());
            }
            if outcome == "win"
                && let Some((_, slug)) = FACTIONS
                    .iter()
                    .find(|(name, _)| *name == p.civilization.name)
            {
                awards.push(format!("win-{slug}"));
            }
            SeatAchievements {
                seat: p.index,
                earned: awards,
                outcome: outcome.into(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::advance::Advance;
    use crate::city::City;
    use crate::events::EventOrigin;
    use crate::game::{GameContext, GameOptions};
    use crate::game_setup::{GameSetupBuilder, setup_game};
    use crate::leader::Leader;
    use crate::objective_card::CompletedObjective;
    use crate::playing_actions::PlayingAction;
    use crate::special_advance::SpecialAdvance;
    use crate::victory_points::{SpecialVictoryPoints, VictoryPointAttribution};
    use enumset::EnumSet;
    use std::collections::HashSet;

    fn setup(length: GameLength) -> Game {
        setup_variant(length, GameVariant::Standard)
    }

    fn setup_variant(length: GameLength, variant: GameVariant) -> Game {
        let mut game = setup_game(
            &GameSetupBuilder::new(2)
                .seed("achievements-tests".into())
                .assigned_civilizations(vec!["Rome".into(), "Greece".into()])
                .options(GameOptions {
                    length,
                    variant,
                    ..GameOptions::default()
                })
                .build(),
        );
        for p in &mut game.players {
            p.objective_cards.clear();
        }
        game.starting_player_index = 0;
        game.current_player_index = 1;
        game.age = game.options.length.ages();
        game.round = 3;
        game
    }

    fn finish(game: Game) -> Game {
        let game = crate::game_api::execute(game, Action::Playing(PlayingAction::EndTurn), 1);
        assert_eq!(game.state, GameState::Finished);
        game
    }

    fn has(game: &Game, seat: usize, id: &str) -> bool {
        achievements(game)
            .iter()
            .any(|p| p.seat == seat && p.earned.iter().any(|a| a == id))
    }

    fn set_score(game: &mut Game, seat: usize, target: f32) {
        let delta = target - game.player(seat).victory_points(game);
        game.player_mut(seat)
            .special_victory_points
            .push(SpecialVictoryPoints::new(
                delta,
                EventOrigin::Ability("test score".into()),
                VictoryPointAttribution::Events,
            ));
    }

    #[test]
    fn manifest_and_real_end_turn_for_all_lengths() {
        let manifest = definitions();
        assert_eq!(manifest.revision, 3);
        assert_eq!(manifest.achievements.len(), 41);
        assert_eq!(
            manifest
                .achievements
                .iter()
                .map(|d| &d.id)
                .collect::<HashSet<_>>()
                .len(),
            41
        );
        assert!(
            manifest
                .achievements
                .iter()
                .any(|d| d.name == "Complete Standard")
        );
        for (length, id, _) in LENGTHS {
            let mut game = setup(length);
            set_score(&mut game, 0, 20.0);
            let game = finish(game);
            assert!(has(&game, 0, &format!("complete-{id}")));
            assert!(has(&game, 1, &format!("complete-{id}")));
            assert!(has(&game, 0, &format!("win-{id}")));
            assert!(!has(&game, 1, &format!("win-{id}")));
            assert_eq!(achievements(&game).len(), 2); // excludes barbarians/pirates
        }
    }

    #[test]
    fn ineligible_states_and_dropped_seats() {
        let mut game = setup(GameLength::Standard);
        assert!(achievements(&game).is_empty());
        game.state = GameState::ChooseCivilization;
        assert!(achievements(&game).is_empty());
        game.state = GameState::Playing;
        let mut game = finish(game);
        game.secrets_stripped = true;
        assert!(achievements(&game).is_empty());
        game.secrets_stripped = false;
        set_score(&mut game, 0, 100.0);
        game.dropped_players.push(0);
        assert_eq!(achievements(&game).len(), 1);
        assert!(has(&game, 1, "win-standard"));
        game.dropped_players.push(1);
        assert!(achievements(&game).is_empty());
    }

    #[test]
    fn exact_tie_is_a_draw_but_city_tiebreak_wins() {
        let mut game = finish(setup(GameLength::Standard));
        set_score(&mut game, 0, 20.0);
        set_score(&mut game, 1, 20.0);
        assert_eq!(
            compare_score(game.player(0), game.player(1), &game),
            Ordering::Equal
        );
        assert!(!has(&game, 0, "win-standard"));
        assert!(!has(&game, 1, "win-standard"));
        assert!(achievements(&game).iter().all(|p| p.outcome == "draw"));
        let pos = game.player(0).cities[0].position;
        game.player_mut(0).cities.push(City::new(0, pos));
        let score = game.player(0).victory_points(&game);
        set_score(&mut game, 1, score);
        assert!(has(&game, 0, "win-standard"));
        assert!(!has(&game, 1, "win-standard"));
        assert_eq!(achievements(&game)[0].outcome, "win");
        assert_eq!(achievements(&game)[1].outcome, "loss");
    }

    #[test]
    fn score_thresholds_are_inclusive() {
        for (score, thirty, forty) in [
            (29.5, false, false),
            (30.0, true, false),
            (39.5, true, false),
            (40.0, true, true),
            (45.0, true, true),
        ] {
            let mut game = finish(setup(GameLength::Standard));
            set_score(&mut game, 0, score);
            assert_eq!(has(&game, 0, "score-30"), thirty);
            assert_eq!(has(&game, 0, "score-40"), forty);
        }
    }

    #[test]
    fn final_state_counts_not_special_advances_or_owned_wonders() {
        let mut game = finish(setup(GameLength::Standard));
        for count in [0, 1, 2] {
            let p = game.player_mut(0);
            p.advances = EnumSet::<Advance>::all().iter().take(19 + count).collect();
            p.special_advances.insert(SpecialAdvance::Aqueduct);
            p.completed_objectives = (0..2 + count)
                .map(|i| CompletedObjective {
                    card: i as u8,
                    name: format!("test {i}"),
                })
                .collect();
            p.wonders_owned = [Wonder::Colosseum, Wonder::Pyramids].into_iter().collect();
            p.wonders_built =
                [Wonder::Colosseum, Wonder::Pyramids, Wonder::GreatGardens][..1 + count].to_vec();
            p.captured_leaders = [Leader::Alexander, Leader::Caesar][..count].to_vec();
            for id in [
                "twenty-advances",
                "three-objectives",
                "two-wonders-built",
                "captured-leader",
            ] {
                assert_eq!(has(&game, 0, id), count >= 1, "{id}, offset {count}");
            }
        }
    }

    #[test]
    fn faction_catalog_matches_every_playable_civilization_and_wire_shape() {
        let playable: HashSet<_> = crate::content::civilizations::get_all_uncached()
            .into_iter()
            .filter(crate::civilization::Civilization::can_choose)
            .map(|c| c.name)
            .collect();
        assert_eq!(
            playable,
            FACTIONS.iter().map(|(name, _)| (*name).into()).collect()
        );
        let manifest = serde_json::to_value(definitions()).unwrap();
        assert_eq!(manifest["revision"], 3);
        assert_eq!(manifest.as_object().unwrap().len(), 2);
        let entries = manifest["achievements"].as_array().unwrap();
        assert_eq!(entries.len(), 41);
        for entry in entries {
            let faction = FACTIONS
                .iter()
                .find(|(_, slug)| entry["id"] == format!("win-{slug}"));
            if let Some((name, slug)) = faction {
                assert_eq!(*slug, name.to_lowercase());
                assert_eq!(entry["name"], format!("Win as {name}"));
                assert_eq!(
                    entry["icon"],
                    serde_json::json!({
                        "type": "asset", "key": format!("achievement-faction-{slug}")
                    })
                );
                assert_eq!(entry.as_object().unwrap().len(), 4);
            } else {
                assert_eq!(
                    entry["icon"]["key"],
                    format!("achievement-{}", entry["id"].as_str().unwrap())
                );
                assert_eq!(entry.as_object().unwrap().len(), 4);
            }
        }
        assert!(!manifest.to_string().contains("four-cities"));
    }

    #[test]
    fn every_faction_requires_exact_name_and_a_win_not_loss_or_draw() {
        for civilization in crate::content::civilizations::get_all_uncached()
            .into_iter()
            .filter(crate::civilization::Civilization::can_choose)
        {
            let name = civilization.name.clone();
            let mut game = finish(setup(GameLength::Standard));
            game.player_mut(0).civilization = civilization;
            for (score, outcome) in [(30.0, "win"), (10.0, "loss"), (20.0, "draw")] {
                set_score(&mut game, 0, score);
                set_score(&mut game, 1, 20.0);
                let results = achievements(&game);
                assert_eq!(results[0].outcome, outcome, "{name}");
                for result in &results {
                    for (faction, slug) in FACTIONS {
                        assert_eq!(
                            result.earned.contains(&format!("win-{slug}")),
                            result.outcome == "win"
                                && game.player(result.seat).civilization.name == faction,
                            "{name}, {outcome}, {slug}"
                        );
                    }
                }
                let json = serde_json::to_value(&results).unwrap();
                assert_eq!(
                    json[0],
                    serde_json::json!({
                        "seat": 0, "earned": results[0].earned, "outcome": outcome
                    })
                );
                assert!(!json.to_string().contains("four-cities"));
            }
            set_score(&mut game, 0, 30.0);
            game.player_mut(0).civilization.name = name.to_lowercase();
            assert!(!has(&game, 0, &format!("win-{}", name.to_lowercase())));
        }
    }

    #[test]
    fn seven_current_cities_is_the_legal_capacity_boundary() {
        let mut game = finish(setup(GameLength::Standard));
        let positions: Vec<_> = game.map.tiles.keys().copied().take(7).collect();
        assert_eq!(positions.len(), 7);
        assert_eq!(game.player(0).city_limit(), 7);
        for (count, earned) in [(6, false), (7, true), (6, false)] {
            game.player_mut(0).cities = positions
                .iter()
                .take(count)
                .map(|pos| City::new(0, *pos))
                .collect();
            assert_eq!(has(&game, 0, "seven-cities"), earned);
            assert!(
                !serde_json::to_string(&achievements(&game))
                    .unwrap()
                    .contains("four-cities")
            );
        }
    }

    #[test]
    fn builder_real_end_turn_awards_losing_seat_but_not_unfinished_or_stripped() {
        let mut game = setup_variant(GameLength::Standard, GameVariant::Builder);
        set_score(&mut game, 0, 40.0);
        assert!(achievements(&game).is_empty());
        let mut game = finish(game);
        let results = achievements(&game);
        assert_eq!(results[1].outcome, "loss");
        assert!(has(&game, 0, "complete-builder"));
        assert!(has(&game, 1, "complete-builder"));
        assert!(has(&game, 0, "win-rome"));
        assert!(!has(&game, 1, "win-greece"));
        game.secrets_stripped = true;
        assert!(achievements(&game).is_empty());
        assert!(!has(
            &finish(setup(GameLength::Standard)),
            0,
            "complete-builder"
        ));
    }

    #[test]
    fn wonder_catalog_matches_real_registry_and_standard_names() {
        let registered: HashSet<_> = crate::content::wonders::get_all_uncached()
            .into_iter()
            .map(|info| info.wonder)
            .filter(|wonder| *wonder != Wonder::Hidden)
            .collect();
        assert_eq!(
            registered,
            WONDERS.iter().map(|(wonder, _, _)| *wonder).collect()
        );
        assert!(
            !WONDERS
                .iter()
                .any(|(wonder, _, _)| *wonder == Wonder::Hidden)
        );
        let manifest = definitions();
        for (_, id, name) in WONDERS {
            let entry = manifest
                .achievements
                .iter()
                .find(|entry| entry.id == id)
                .unwrap();
            assert_eq!(entry.name, format!("{name}: Recorded as Built"));
            assert_eq!(
                entry.description,
                format!(
                    "Finish with {name} recorded as built by your civilization. Destroyed wonders and merely owning captured wonders do not count."
                )
            );
            assert_eq!(
                entry.icon,
                Some(Icon::Asset {
                    key: format!("achievement-{id}")
                })
            );
        }
        assert!(
            manifest.achievements.iter().all(
                |entry| !entry.name.contains("Normal") && !entry.description.contains("Normal")
            )
        );
    }

    #[test]
    fn every_wonder_requires_surviving_built_record_not_ownership_or_a_win() {
        for (wonder, id, _) in WONDERS {
            for builder in [0, 1] {
                let owner = 1 - builder;
                let mut game = finish(setup(GameLength::Standard));
                assert!(!has(&game, builder, id));
                assert!(!has(&game, owner, id));
                // Capturing ownership alone is not a construction record.
                let position = game.player(owner).cities[0].position;
                let event_player = crate::events::EventPlayer::new(
                    owner,
                    EventOrigin::Ability("wonder capture and destruction regression".into()),
                );
                crate::wonder::gain_wonder(&mut game, &event_player, wonder, position);
                crate::wonder::init_wonder(&mut game, owner, wonder);
                assert!(!has(&game, owner, id));
                game.player_mut(builder).wonders_built.push(wonder);
                set_score(&mut game, builder, 10.0);
                set_score(&mut game, owner, 100.0);
                assert_eq!(achievements(&game)[builder].outcome, "loss");
                assert!(has(&game, builder, id), "{id}, builder {builder}");
                assert!(!has(&game, owner, id));
                for (_, other_id, _) in WONDERS {
                    assert_eq!(has(&game, builder, other_id), other_id == id);
                }
                assert!(!has(&game, builder, "two-wonders-built"));
                crate::wonder::destroy_wonder(&mut game, &event_player, wonder, position);
                assert!(!game.player(builder).wonders_built.contains(&wonder));
                assert!(!has(&game, builder, id));
                assert!(!has(&game, owner, id));
            }
        }
    }

    #[test]
    fn destroyed_wonder_no_longer_counts_in_final_built_record() {
        let mut game = finish(setup(GameLength::Standard));
        let position = game.player(0).cities[0].position;
        let owner = crate::events::EventPlayer::new(
            0,
            EventOrigin::Ability("wonder destruction regression".into()),
        );
        game.player_mut(0).wonders_built = vec![Wonder::Pyramids, Wonder::Colosseum];
        crate::wonder::gain_wonder(&mut game, &owner, Wonder::Pyramids, position);
        assert!(has(&game, 0, "two-wonders-built"));
        crate::wonder::destroy_wonder(&mut game, &owner, Wonder::Pyramids, position);
        assert_eq!(game.player(0).wonders_built, vec![Wonder::Colosseum]);
        assert!(!has(&game, 0, "two-wonders-built"));
    }

    #[test]
    fn saved_end_game_fixture_and_nonwinning_feats() {
        let data: crate::game_data::GameData = serde_json::from_str(include_str!(
            "../tests/test_games/status_phase/end_game.json"
        ))
        .expect("valid fixture");
        let cache = crate::cache::Cache::new(&data.options);
        let game = Game::from_data(data, cache, GameContext::Play);
        assert!(achievements(&game).is_empty());
        let mut game = crate::game_api::execute(game, Action::Playing(PlayingAction::EndTurn), 0);
        assert_eq!(game.state, GameState::Finished);
        assert_eq!(achievements(&game).len(), 2);
        set_score(&mut game, 0, 40.0);
        set_score(&mut game, 1, 100.0);
        let results = achievements(&game);
        assert_eq!(results[0].outcome, "loss");
        assert!(has(&game, 0, "score-30"));
        assert!(has(&game, 0, "score-40"));
        assert!(!has(&game, 0, "win-standard"));
    }

    #[test]
    fn deterministic_read_only_and_save_load_stable() {
        let mut game = finish(setup(GameLength::Standard));
        set_score(&mut game, 0, 40.0);
        let before = serde_json::to_value(game.cloned_data()).expect("serialize game");
        let awards = achievements(&game);
        assert_eq!(awards, achievements(&game));
        assert_eq!(
            before,
            serde_json::to_value(game.cloned_data()).expect("serialize game")
        );
        let data = serde_json::from_value(before).expect("deserialize game");
        let restored = Game::from_data(data, game.cache.clone(), GameContext::Play);
        assert_eq!(awards, achievements(&restored));
        let json = serde_json::to_value(&awards).expect("serialize awards");
        assert_eq!(json[0]["seat"], 0);
        assert!(json[0]["earned"].is_array());
    }
}
