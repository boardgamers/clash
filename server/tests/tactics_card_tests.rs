use crate::common::{TestAction, move_action};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::new("tactics_cards");

#[test]
fn test_peltasts() {
    JSON.test(
        "peltasts",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))),
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
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))),
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
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
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
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
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
fn test_high_ground() {
    JSON.test(
        "high_ground",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    9,
                )])),
            ),
        ],
    );
}

#[test]
fn test_surprise() {
    JSON.test(
        "surprise",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    10,
                )])),
            ),
        ],
    );
}

#[test]
fn test_siege() {
    JSON.test(
        "siege",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    11,
                )])),
            ),
        ],
    );
}

#[test]
fn test_scout() {
    JSON.test(
        "scout",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    23,
                )])),
            ),
        ],
    );
}

#[test]
fn test_martyr() {
    JSON.test(
        "martyr",
        vec![
            TestAction::undoable(0, move_action(vec![7, 8], Position::from_offset("D2")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    6,
                )])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    24,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![7])))
                .without_json_comparison(),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![2]))),
        ],
    );
}

#[test]
fn test_archers() {
    JSON.test(
        "archers",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    26,
                )])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectUnits(vec![1]))),
        ],
    );
}

#[test]
fn test_flanking() {
    JSON.test(
        "flanking",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    42,
                )])),
            ),
        ],
    );
}
