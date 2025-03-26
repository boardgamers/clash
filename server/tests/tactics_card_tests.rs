use crate::common::{move_action, TestAction};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::custom_phase_actions::EventResponse;
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::new("tactics_cards");

#[test]
fn test_peltasts() {
    JSON.test(
        "peltasts",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_encircled() {
    JSON.test(
        "encircled",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    2,
                )])),
            ),
        ],
    );
}

#[test]
fn test_wedge_formation() {
    JSON.test(
        "wedge_formation",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    5,
                )])),
            ),
        ],
    );
}

#[test]
fn test_high_morale() {
    JSON.test(
        "high_morale",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    6,
                )])),
            ),
        ],
    );
}

#[test]
fn test_heavy_resistance() {
    JSON.test(
        "heavy_resistance",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    7,
                )])),
            ),
        ],
    );
}

#[test]
fn test_elevated_position() {
    JSON.test(
        "elevated_position",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            ).without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    9,
                )])),
            ),
        ],
    );
}
