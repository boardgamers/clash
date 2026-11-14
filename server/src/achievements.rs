//! Read-only, revisioned achievements derived from authoritative final game state.
use crate::game::{Game, GameLength, GameState};
use crate::victory_points::compare_score;
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
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SeatAchievements {
    pub seat: usize,
    pub earned: Vec<String>,
    pub outcome: String,
}

const LENGTHS: [(GameLength, &str, &str); 5] = [
    (GameLength::Shorter, "shorter", "Shorter"),
    (GameLength::Standard, "standard", "Standard (Normal)"),
    (GameLength::RollToEnd, "roll-to-end", "Roll to End"),
    (
        GameLength::WonderfulEnding,
        "wonderful-ending",
        "Wonderful Ending",
    ),
    (GameLength::Epic, "epic", "Epic"),
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
        "four-cities",
        "Four Cities",
        "Finish a game owning at least 4 cities in the final state.",
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
    let mut definitions = Vec::with_capacity(17);
    for (_, id, name) in LENGTHS {
        definitions.push(Definition {
            id: format!("complete-{id}"),
            name: format!("Complete {name}"),
            description: format!("Finish a {name} game without dropping out."),
        });
        definitions.push(Definition {
            id: format!("win-{id}"),
            name: format!("Win {name}"),
            description: format!("Finish a {name} game as the sole highest-ranked non-dropped human seat, including score tiebreakers; tied games do not count as wins."),
        });
    }
    definitions.extend(FEATS.map(|(id, name, description)| Definition {
        id: id.into(),
        name: name.into(),
        description: description.into(),
    }));
    Manifest {
        revision: 1,
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
                p.cities.len() >= 4,
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
    use crate::wonder::Wonder;
    use enumset::EnumSet;
    use std::collections::HashSet;

    fn setup(length: GameLength) -> Game {
        let mut game = setup_game(
            &GameSetupBuilder::new(2)
                .seed("achievements-tests".into())
                .assigned_civilizations(vec!["Rome".into(), "Greece".into()])
                .options(GameOptions {
                    length,
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
        assert_eq!(manifest.revision, 1);
        assert_eq!(manifest.achievements.len(), 17);
        assert_eq!(
            manifest
                .achievements
                .iter()
                .map(|d| &d.id)
                .collect::<HashSet<_>>()
                .len(),
            17
        );
        assert!(
            manifest
                .achievements
                .iter()
                .any(|d| d.name == "Complete Standard (Normal)")
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
        let pos = game.player(0).cities[0].position;
        for count in [0, 1, 2] {
            let p = game.player_mut(0);
            p.cities = (0..3 + count).map(|_| City::new(0, pos)).collect();
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
                "four-cities",
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
