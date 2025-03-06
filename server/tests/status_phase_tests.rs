use crate::common::{test_action, test_actions, TestAction};
use server::action::Action;
use server::content::custom_phase_actions::CurrentEventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::status_phase::{ChangeGovernment, ChangeGovernmentType};

mod common;

#[test]
fn test_end_game() {
    test_actions(
        "end_game",
            vec![TestAction::not_undoable(
            0,
            Action::Playing(PlayingAction::EndTurn),
        )],
    );
}

#[test]
fn test_free_advance() {
    test_actions(
        "free_advance",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
        ],
    );
}

#[test]
fn test_wrong_status_phase_action() {
    test_action(
        "illegal_free_advance",
        Action::Response(CurrentEventResponse::SelectPositions(vec![])),
        0,
        false,
        true,
    );
}

#[test]
fn test_raze_city() {
    test_action(
        "raze_city",
        Action::Response(CurrentEventResponse::SelectPositions(vec![
            Position::from_offset("A1"),
        ])),
        0,
        false,
        false,
    );
}

#[test]
fn test_raze_city_decline() {
    test_action(
        "raze_city_decline",
        Action::Response(CurrentEventResponse::SelectPositions(vec![])),
        0,
        false,
        false,
    );
}

#[test]
fn test_determine_first_player() {
    test_action(
        "determine_first_player",
        Action::Response(CurrentEventResponse::SelectPlayer(1)),
        0,
        false,
        false,
    );
}

#[test]
fn test_change_government() {
    test_action(
        "change_government",
        Action::Response(CurrentEventResponse::ChangeGovernmentType(
            ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                new_government: String::from("Theocracy"),
                additional_advances: vec![String::from("Devotion")],
            }),
        )),
        0,
        false,
        false,
    );
}
