use crate::common::{JsonTest, TestAction};
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;

mod common;

const JSON: JsonTest = JsonTest::child("objective_cards", "status_phase");

#[test]
fn test_large_civ() {
    JSON.test(
        "large_civ",
        vec![
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(4),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(6),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(24),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(1),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(11),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(5),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(9),
                ])),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(7),
                ])),
            ),
        ],
    )
}

#[test]
fn test_colony() {
    // Game ends after this step
    JSON.test(
        "colony",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(17),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(32),
                ])),
            ),
        ],
    )
}

#[test]
fn test_standing_army() {
    JSON.test(
        "standing_army",
        vec![
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(14),
                ])),
            ),
        ],
    )
}

#[test]
fn colony_does_not_count_distance_through_unexplored_regions() {
    let mut game = JSON.load_game("colony");
    for pos in ["D4", "D5", "D6"] {
        game.map.tiles.insert(server::position::Position::from_offset(pos), server::map::Terrain::Unexplored);
    }
    let game = server::game_api::execute(game, Action::Playing(PlayingAction::EndTurn), 0);
    let event = serde_json::to_value(game.current_event()).unwrap();
    let choices = event["handler"]["request"]["SelectHandCards"]["choices"].as_array().unwrap();
    assert!(!choices.contains(&serde_json::json!({"ObjectiveCard": 17})));
    assert!(choices.contains(&serde_json::json!({"ObjectiveCard": 32})));
}
